use std::{thread, time};
use tokio::sync::mpsc::*;

pub enum TickFanoutCommand {
    NewRecipient { sender: Sender<()> }
}

pub struct TickFanoutCommander {
    tx: Sender<TickFanoutCommand>,
}

impl TickFanoutCommander {
    pub fn new(tick: Receiver<()>) -> TickFanoutCommander {
	let recipients: Vec<Sender<()>> = Vec::new();
	let (command_tx, mut command_rx) = channel(1);
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
    pub async fn send_command(self, command: TickFanoutCommand) -> TickFanoutCommander {
	self.tx.send(command).await;
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
	    tokio::select! {
		command = command_rx.recv() => {
		    if let Some(c) = command {
			self.process_command(c, &mut channels);

//			thread::sleep(time::Duration::from_millis(10));
		    }
		}
		_ = channels.tick.recv() => {
		    self.fanout_process(&mut channels).await;
//		    thread::sleep(time::Duration::from_millis(1));
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

    async fn fanout_process(
	&self,
	channels: &mut TickFanoutChannels
    ) {
	for recipient in &channels.recipients {
		recipient.send(()).await;
	}
    }
}
