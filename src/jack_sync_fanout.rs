use std::{thread, time};
use tokio::sync::mpsc::*;
use jack::jack_sys as j;
use std::mem::MaybeUninit;
use crate::constants::*;

#[derive(Copy, Clone)]
pub struct JackSyncFanoutMessage {
    pub pos_frame: usize,
    pub framerate: usize,
    pub beats_per_bar: usize,
    pub beat: usize,
    pub nframes: usize,
    pub next_beat_frame: usize,
    pub beat_this_cycle: bool
}


pub enum JackSyncFanoutCommand {
    NewRecipient { sender: Sender<JackSyncFanoutMessage> }
}

pub struct JackSyncFanoutCommander {
    tx: Sender<JackSyncFanoutCommand>,
}

impl JackSyncFanoutCommander {
    pub fn new(tick: Receiver<()>, jack_client_addr: usize) -> JackSyncFanoutCommander {
	let recipients: Vec<Sender<()>> = Vec::new();
	let (command_tx, mut command_rx) = channel(1);
	let mut recipients = Vec::new();
	let channels = JackSyncFanoutChannels {
	    tick,
	    recipients
	};
	
	let fan = JackSyncFanout::new(jack_client_addr);
	unsafe {
            tokio::task::spawn(async move {
		fan.start(command_rx, channels, ).await;
	    });
	}

	
	
	JackSyncFanoutCommander {
	    tx: command_tx
	}
    }
    pub async fn send_command(&self, command: JackSyncFanoutCommand) {
	self.tx.send(command).await;
    }
}


#[derive(Debug)]
pub struct JackSyncFanoutChannels {
    tick: Receiver<()>,
    recipients: Vec<Sender<JackSyncFanoutMessage>>,
    
}

unsafe impl Send for JackSyncFanoutChannels {}
    

pub struct JackSyncFanout {
    jack_client_addr: usize,
    sync: st_sync::client::Client,
    next_beat_frame: usize,
    last_frame: usize
}

impl JackSyncFanout {
    pub fn new(jack_client_addr: usize) -> JackSyncFanout {
	let sync = st_sync::client::Client::new();
	let mut next_beat_frame = 0;
	loop {
	    //first beat frame
	    if let Ok(frame) = sync.try_recv_next_beat_frame() {
		dbg!(frame);
		next_beat_frame = frame as usize;
		break;
	    }
	    thread::sleep(time::Duration::from_millis(ASYNC_COMMAND_LATENCY));
	}

	let last_frame = 0;
	JackSyncFanout { jack_client_addr, sync, next_beat_frame, last_frame}
    }

    async fn start(
	mut self,
	mut command_rx: Receiver<JackSyncFanoutCommand>,
	mut channels: JackSyncFanoutChannels
    ) {
	loop {
	    tokio::select! {
		command = command_rx.recv() => {
		    if let Some(c) = command {
			self.process_command(c, &mut channels);

		    }
		}
		_ = channels.tick.recv() => {
		    self.fanout_process(&mut channels);
		}
	    }
	}
    }
    
    fn process_command(
	&self,
	command: JackSyncFanoutCommand,
	channels: &mut JackSyncFanoutChannels
    ) {
	match command {
	    JackSyncFanoutCommand::NewRecipient { sender } => {
		channels.recipients.push(sender);
	    }
	}

    }

    fn fanout_process(
	&mut self,
	channels: &mut JackSyncFanoutChannels
    ) {
	let mut pos = MaybeUninit::uninit().as_mut_ptr();
	let mut msg = JackSyncFanoutMessage{
	    pos_frame: 0,
	    framerate: 48000,
	    beats_per_bar: 0,
	    beat: 0,
	    nframes: 0,
	    next_beat_frame: 0,
	    beat_this_cycle: false
	};
	
	let client_pointer: *const j::jack_client_t = std::ptr::from_exposed_addr(self.jack_client_addr);

	unsafe {
	    j::jack_transport_query(client_pointer, pos);
	    msg.pos_frame = (*pos).frame as usize;
	    msg.framerate = (*pos).frame_rate as usize;
	    msg.beats_per_bar = (*pos).beats_per_bar as usize;
	    msg.beat = (*pos).beat as usize;
	}	    
	msg.nframes = msg.pos_frame - self.last_frame;

	if msg.pos_frame >= self.next_beat_frame {
//		println!("checking");
	    if let Ok(frame) = (&self).sync.try_recv_next_beat_frame() {
		self.next_beat_frame = frame as usize;
//		    println!("next beat frame: {}", next_beat_frame);
//		    println!("pos frame: {}", pos_frame);
	    }
	}
	if (((self.last_frame < self.next_beat_frame) &&
	     (self.next_beat_frame <= msg.pos_frame))) ||
	    self.last_frame == 0 {
		msg.beat_this_cycle = true;
	    }	
	
	self.last_frame = msg.pos_frame;
	for recipient in &channels.recipients {
	    //todo. Can't use async send because pos is not Send (breaks task spawning)
	    // can't use blocking send because it crashes thread with "Cannot start a runtime from within a runtime"
	    recipient.try_send(msg);
	}
    }
}
