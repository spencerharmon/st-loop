use std::{thread, time};
use tokio::sync::mpsc::*;
use jack::jack_sys as j;
use std::mem::MaybeUninit;

pub enum JackSyncFanoutCommand {
    NewRecipient { sender: Sender<()> }
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
    pub async fn send_command(self, command: JackSyncFanoutCommand) -> JackSyncFanoutCommander {
	self.tx.send(command).await;
	self
    }
    
    pub fn try_send_command(self, command: JackSyncFanoutCommand) -> JackSyncFanoutCommander {
	self.tx.try_send(command);
	self
    }
}


#[derive(Debug)]
pub struct JackSyncFanoutChannels {
    tick: Receiver<()>,
    recipients: Vec<Sender<()>>,
    
}

unsafe impl Send for JackSyncFanoutChannels {}
    

pub struct JackSyncFanout {
    jack_client_addr: usize
}

impl JackSyncFanout {
    pub fn new(jack_client_addr: usize) -> JackSyncFanout {
	JackSyncFanout { jack_client_addr }
    }

    async fn start(
	self,
	mut command_rx: Receiver<JackSyncFanoutCommand>,
	mut channels: JackSyncFanoutChannels
    ) {
//	let channels_rw = RwLock::new(channels);
//        let mut channels_lock = channels_rw.write().await;
	loop {
	    tokio::select! {
		command = command_rx.recv() => {
		    if let Some(c) = command {
			self.process_command(c, &mut channels);

//			thread::sleep(time::Duration::from_millis(10));
		    }
		}
		_ = channels.tick.recv() => {
		    self.fanout_process(&mut channels);
//		    thread::sleep(time::Duration::from_millis(1));
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
	&self,
	channels: &mut JackSyncFanoutChannels
    ) {
	let mut pos = MaybeUninit::uninit().as_mut_ptr();
	let mut pos_frame = 0;
	let mut framerate = 48000;
	let mut last_frame = pos_frame;
	let mut beats_per_bar = 0;
	let mut beat = 0;
	let client_pointer: *const j::jack_client_t = std::ptr::from_exposed_addr(self.jack_client_addr);
	unsafe {
	    j::jack_transport_query(client_pointer, pos);
	    pos_frame = (*pos).frame as usize;
	    framerate = (*pos).frame_rate as usize;
	    beats_per_bar = (*pos).beats_per_bar as usize;
	    beat = (*pos).beat as usize;
	}	    
	
	for recipient in &channels.recipients {
	    //todo. Can't use async send because pos is not Send (breaks task spawning)
	    // can't use blocking send because it crashes thread with "Cannot start a runtime from within a runtime"
	    recipient.try_send(());
	}
    }
}
