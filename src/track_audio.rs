use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::select;
use tokio::sync::mpsc::{Receiver, Sender, UnboundedSender};
use std::cell::RefCell;
use crossbeam_channel;
use tokio::task::LocalSet;

pub enum TrackAudioCommand {
    NewSeq { channel: Receiver<(f32, f32)> },
    DelLastSeq,
}

struct TrackAudioData {
    command_rx: Receiver<TrackAudioCommand>,
    jack_tick: Receiver<()>,
}

pub struct TrackAudioCombinerCommander {
    tx: Sender<TrackAudioCommand>
}

impl TrackAudioCombinerCommander {
    pub fn new(
	output: (
	    crossbeam_channel::Sender<f32>,
	    crossbeam_channel::Sender<f32>
	),
	jack_tick: Receiver<()>
    ) -> TrackAudioCombinerCommander {
        let (tx, rx) = mpsc::channel(1);
        let sequences = Vec::new();
        let c = TrackAudioChannels {
            jack_tick,
            output,
            sequences
        };
        let t = TrackAudioCombiner::new();
	let local = LocalSet::new();
        local.spawn_local(async move {
	    t.start(rx, c).await;
	});
			   
        TrackAudioCombinerCommander { tx }
    }
    pub fn send_command(self, command: TrackAudioCommand){
        self.tx.send(command);
    }
    
    pub fn try_send_command(self, command: TrackAudioCommand){
        self.tx.try_send(command);
    }
}

struct TrackAudioChannels {
    jack_tick: Receiver<()>,
    output: (
	crossbeam_channel::Sender<f32>,
	crossbeam_channel::Sender<f32>
    ),
    sequences: Vec<Receiver<(f32, f32)>>
}

struct TrackAudioCombiner {}

impl TrackAudioCombiner {
    pub fn new() -> TrackAudioCombiner {
        TrackAudioCombiner { }
    }

    pub async fn start(
        mut self,
        mut command_rx: mpsc::Receiver<TrackAudioCommand>,
        channels: TrackAudioChannels
    ) {
        let channels = RefCell::new(channels);
        
        loop {
            let mut channels_ref = channels.borrow_mut();
            select! {
                command = command_rx.recv() => {
                    if let Some(c) = command {
                        self.process_command(&mut channels_ref, c);
                    }
                },
                _ = channels_ref.jack_tick.recv() => {
                    self.process_sequence_data(&mut channels_ref);
                }
            }
        }
    }

    fn process_command(
        &self,
        channels: &mut TrackAudioChannels,
        command: TrackAudioCommand
    ) {
        match command {
            TrackAudioCommand::NewSeq { channel } => {
                channels.sequences.push(channel);
            }
            TrackAudioCommand::DelLastSeq => {
                channels.sequences.pop();
            }
        }
    }
    fn process_sequence_data(
        &self,
        channels: &mut TrackAudioChannels
    ) {
        let mut buf = Vec::new();
	let mut first = true;
	
	let n = channels.sequences.len();
	let channels = RefCell::new(channels);
        for _ in 0..n {
	    let mut channels_ref = channels.borrow_mut();
	    let mut seq = channels_ref.sequences.pop().unwrap();
	    if first {
		loop {
                    if let Ok(v) = seq.try_recv() {
                        buf.push(v);
		    } else {
			break
		    }
		}
		    first = false;
	    } else {
                if let Ok(v) = seq.try_recv() {
		    for i in 0..buf.len() {
			if let Some(tup) = buf.get_mut(i) {
			    tup.0 = tup.0 + v.0;
			    tup.1 = tup.1 + v.1;
			}
                    }
                }
            }
	    for (l, r) in buf.iter() {
		channels_ref.output.0.send(*l);
		channels_ref.output.1.send(*r);
	    }

	}
    }
}
