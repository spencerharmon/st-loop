use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::select;
use std::collections::VecDeque;
use std::cell::RefCell;
use std::rc::Rc;
use tokio::task::LocalSet;

pub enum TickFanoutCommand {
    NewRecipient { sender: Sender<()> }
}

pub struct TickFanoutCommander {
    tx: Sender<TickFanoutCommand>,
}

impl TickFanoutCommander {
    pub fn new(tick: Receiver<()>) -> TickFanoutCommander {
	let recipients: Vec<Sender<()>> = Vec::new();
	let (command_tx, command_rx) = mpsc::channel(1);

	let recipients = Vec::new();
	let channels = TickFanoutChannels {
	    tick,
	    recipients
	};
	let fan = TickFanout::new();
	let local = LocalSet::new();
        local.spawn_local(async move {
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


pub struct TickFanoutChannels {
    tick: Receiver<()>,
    recipients: Vec<Sender<()>>,
    
}
    

pub struct TickFanout {}

impl TickFanout {
    pub fn new() -> TickFanout {
	TickFanout {}
    }

    pub async fn start(
	self,
	mut rx: Receiver<TickFanoutCommand>,
	channels: TickFanoutChannels
    ) {
	let channels = RefCell::new(channels);
	
	loop {
            let mut channels_ref = channels.borrow_mut();
	    select! {
		command = rx.recv() => {
		    if let Some(c) = command {
			self.process_command(c, &mut channels_ref);
		    }
		}
		fanout = self.fanout_process(&mut channels_ref) => { }
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
	loop {
	    channels.tick.recv().await;
	    for recipient in &channels.recipients {
		recipient.send(());
	    }
	}
    }
}
