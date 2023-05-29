use std::{thread, time};
use tokio::sync::mpsc;
use crossbeam_channel::*;
use jack::jack_sys as j;
use std::mem::MaybeUninit;
use crate::constants::*;
use crate::jack_sync_fanout::*;

pub enum AudioInCommand {
    RerouteTrack {
	track: usize,
	recipient: Sender<(f32, f32)>
    }
}

pub struct AudioInSwitchCommander {
    tx: mpsc::Sender<AudioInCommand>
}

impl AudioInSwitchCommander {
    pub fn new(
	inputs: Vec<Receiver<(f32, f32)>>,
	jack_sync_rx: mpsc::Receiver<JackSyncFanoutMessage>
    ) -> AudioInSwitchCommander {
	let (tx, mut rx) = mpsc::channel(AUDIO_TRACK_COUNT);
	let mut switch = AudioInSwitch::new(inputs);
	tokio::spawn(async move {
	    switch.start(rx, jack_sync_rx).await;
	});
	
	AudioInSwitchCommander {
	    tx
	}
    }
    pub async fn send_command(self, command: AudioInCommand) -> AudioInSwitchCommander{
	self.tx.send(command).await;
	self
    }
}

struct AudioInSwitch {
    input_recipient_map: Vec<(Receiver<(f32, f32)>, Option<Sender<(f32,f32)>>)>
}

impl AudioInSwitch {
    fn new(
	inputs: Vec<Receiver<(f32, f32)>>
    ) -> AudioInSwitch {
	let mut input_recipient_map = Vec::new();
	for i in inputs {
	    input_recipient_map.push((i, None));
	}

	AudioInSwitch {
	    input_recipient_map
	}
    }
    async fn start(
	&mut self,
	mut cmd_rx: mpsc::Receiver<AudioInCommand>,
	mut jack_sync_rx: mpsc::Receiver<JackSyncFanoutMessage>
    ) {
	loop {
	    tokio::select!{
		cmd_o = cmd_rx.recv() => {
		    if let Some(cmd) = cmd_o {
			match cmd {
			    AudioInCommand::RerouteTrack {
				track,
				recipient
			    } => {
				println!("new recipient --------------------------------");
				let (_, ref mut recipient_o) = self.input_recipient_map.get_mut(track).unwrap();
				*recipient_o = Some(recipient);
				dbg!(&self.input_recipient_map);
			    }
			}
		    }
		}
		_ = jack_sync_rx.recv() => {
		    self.process_audio();
		}
	    }
	}
    }
    fn process_audio(&self) {
//	dbg!(&self.input_recipient_map);
	for (input, recipient_o) in &self.input_recipient_map {
	    if let Some(recipient) = recipient_o {
		
		let (l, r) = input.recv().unwrap();
//		dbg!(l);
		recipient.send((l, r));
	    }
	}
    }
}
