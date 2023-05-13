use crossbeam_channel::*;
use crossbeam;
use std::{thread, time};

pub enum TickFanoutCommand {
    NewRecipient { sender: Sender<()> }
}

pub struct TickFanoutCommander {
    tx: Sender<TickFanoutCommand>,
}

impl TickFanoutCommander {
    pub fn new(tick: Receiver<()>) -> TickFanoutCommander {
	let recipients: Vec<Sender<()>> = Vec::new();
	let (command_tx, mut command_rx) = bounded(1);
	let mut recipients = Vec::new();
	let channels = TickFanoutChannels {
	    tick,
	    recipients
	};
	
	let fan = TickFanout::new();
        tokio::task::spawn(async move {
	    fan.start(command_rx, channels).await;
	});

	
	
	TickFanoutCommander {
	    tx: command_tx
	}
    }
    pub fn send_command(self, command: TickFanoutCommand) -> TickFanoutCommander {
	self.tx.send(command);
	self
    }
    
    pub fn try_send_command(self, command: TickFanoutCommand) -> TickFanoutCommander {
	self.tx.try_send(command);
	self
    }
}


#[derive(Debug)]
pub struct TickFanoutChannels {
    tick: Receiver<()>,
    recipients: Vec<Sender<()>>,
    
}

unsafe impl Send for TickFanoutChannels {}
    

pub struct TickFanout {}

impl TickFanout {
    pub fn new() -> TickFanout {
	TickFanout {}
    }

    async fn start(
	self,
	mut command_rx: Receiver<TickFanoutCommand>,
	mut channels: TickFanoutChannels
    ) {
//	let channels_rw = RwLock::new(channels);
//        let mut channels_lock = channels_rw.write().await;
	loop {
	    crossbeam::select! {
		recv(command_rx) -> command => {
		    if let Ok(c) = command {
			self.process_command(c, &mut channels);

			thread::sleep(time::Duration::from_millis(10));
		    }
		}
		recv(channels.tick) -> _ => {
		    self.fanout_process(&mut channels);
		    thread::sleep(time::Duration::from_millis(1));
		}
	    }
	}
    }
    
    fn process_command(
	&self,
	command: TickFanoutCommand,
	channels: &mut TickFanoutChannels
    ) {
	match command {
	    TickFanoutCommand::NewRecipient { sender } => {
		channels.recipients.push(sender);
	    }
	}

    }

    fn fanout_process(
	&self,
	channels: &mut TickFanoutChannels
    ) {
	for recipient in &channels.recipients {
		recipient.send(());
	}
    }
}
