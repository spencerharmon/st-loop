use std::cell::RefCell;
use crossbeam_channel::*;
use std::{thread, time};
use tokio::sync::{RwLock, mpsc};
use std::sync::Mutex;
use std::sync::Arc;
use crate::jack_sync_fanout::*;

#[derive(Debug)]
pub enum TrackAudioCommand {
    NewSeq { channel: Receiver<(f32, f32)> },
    DelLastSeq,
    Play,
    Stop
}

struct TrackAudioData {
    command_rx: mpsc::Receiver<TrackAudioCommand>,
    jack_tick: mpsc::Receiver<JackSyncFanoutMessage>,
}

pub struct TrackAudioCombinerCommander {
    tx: mpsc::Sender<TrackAudioCommand>
}

impl TrackAudioCombinerCommander {
    pub fn new(
	output: Sender<(f32, f32)>,
	jack_tick: mpsc::Receiver<JackSyncFanoutMessage>
    ) -> TrackAudioCombinerCommander {
        let (tx, rx) = mpsc::channel(1);
        let mut sequences = Vec::new();

        let t = TrackAudioCombiner::new(
	    sequences,
	    jack_tick,
	    output,
	    rx
	);
        tokio::task::spawn(async move {
	    t.start().await;
	});

        TrackAudioCombinerCommander { tx }
    }
    pub async fn send_command(&self, command: TrackAudioCommand) -> &TrackAudioCombinerCommander {
        self.tx.send(command).await;
	self
    }
    
    pub fn try_send_command(self, command: TrackAudioCommand) -> TrackAudioCombinerCommander {
        self.tx.try_send(command);
	self
    }
}


struct TrackAudioCombiner {
    sequences: Vec<Receiver<(f32, f32)>>,
    playing: bool,
    jack_tick: mpsc::Receiver<JackSyncFanoutMessage>,
    output: Sender<(f32, f32)>,
    command_rx: mpsc::Receiver<TrackAudioCommand>,
}

impl TrackAudioCombiner {
    pub fn new(sequences: Vec<Receiver<(f32, f32)>>,
	       jack_tick: mpsc::Receiver<JackSyncFanoutMessage>,
	       output: Sender<(f32, f32)>,
               command_rx: mpsc::Receiver<TrackAudioCommand>,
    ) -> TrackAudioCombiner {
	let mut playing = false;
        TrackAudioCombiner { sequences, playing, jack_tick, output, command_rx }
    }

    pub async fn start(mut self) {
	loop {
	    tokio::select! {
		command = self.command_rx.recv() => {
		    if let Some(c) = command {
			println!("processing command: {:?}", c);
			self.process_command(c);
		    }
		}
		Some(msg) = self.jack_tick.recv() => {
		    self.process_sequence_data(msg.nframes/2);
		}
	    }
	}
    }

    fn process_sequence_data(&mut self, nframes: usize) {
	let mut counter = 0;
	loop {
	    let mut tup = (0f32,0f32);
	    let mut data = false;
	    for seq in self.sequences.iter_mut() {
		if let Ok(v) = seq.try_recv() {
		    data = true;
		    tup.0 = tup.0 + v.0;
		    tup.1 = tup.1 + v.1;
		}
	    }
	    if data {
		self.output.send(tup);
	    }
	    counter = counter + 1;
	    if counter >= nframes {
		break
	    }
	}
    }
    fn process_command(
	&mut self,
	command: TrackAudioCommand
    ) {
	match command {
	    TrackAudioCommand::NewSeq { channel } => {
		self.sequences.push(channel);
	    }
	    TrackAudioCommand::DelLastSeq => {
		println!("DelLastSeq");
		self.sequences.pop();
	    }
	    TrackAudioCommand::Play => {
		self.playing = true;
	    }
	    TrackAudioCommand::Stop => {
		self.playing = false;
	    }
	}
    }
}


