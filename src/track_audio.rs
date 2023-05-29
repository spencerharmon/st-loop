use std::cell::RefCell;
use crossbeam_channel::*;
use std::{thread, time};
use tokio::sync::{RwLock, mpsc};
use std::sync::Mutex;
use std::sync::Arc;
use crate::jack_sync_fanout::*;

//todo remove me
pub fn sine_wave_generator(freq: &f32, length: usize, sample_rate: u16) -> Vec<f32> {
    let mut ret = vec![0f32; length.into()];
    let samples_per_period =  sample_rate / *freq as u16;
    for i in 0..length {
        ret[i as usize] = (2f32 * std::f32::consts::PI * i as f32 / samples_per_period as f32).sin();

    }
	ret
}

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
        let sequences = Vec::new();
        let c = TrackAudioChannels {
            jack_tick,
            output
        };
	let s = TrackAudioState {
	    playing: false,
	    sequences
	};
        let t = TrackAudioCombiner::new();
        tokio::task::spawn(async move {
	    t.start(rx, c, s).await;
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

struct TrackAudioChannels {
    jack_tick: mpsc::Receiver<JackSyncFanoutMessage>,
    output: Sender<(f32, f32)>,
}

#[derive(Debug)]
struct TrackAudioState {
    sequences: Vec<Receiver<(f32, f32)>>,
    playing: bool
}

fn process_command(
    state: &mut TrackAudioState,
    command: TrackAudioCommand
) {
    match command {
	TrackAudioCommand::NewSeq { channel } => {
	    state.sequences.push(channel);
	}
	TrackAudioCommand::DelLastSeq => {
	    state.sequences.pop();
	}
	TrackAudioCommand::Play => {
	    state.playing = true;
	}
	TrackAudioCommand::Stop => {
	    state.playing = false;
	}
    }
    dbg!(&state);
}

struct TrackAudioCombiner {}

impl TrackAudioCombiner {
    pub fn new() -> TrackAudioCombiner {
        TrackAudioCombiner { }
    }

    pub async fn start(
        mut self,
        mut command_rx: mpsc::Receiver<TrackAudioCommand>,
        mut channels: TrackAudioChannels,
	mut state: TrackAudioState
    ) {

	let state_arc = Arc::new(Mutex::new(state));

    	let s_clone1 = state_arc.clone();
    	let s_clone2 = state_arc.clone();


	loop {
	    tokio::select! {
		command = command_rx.recv() => {
		    if let Some(c) = command {
			let mut s = s_clone1.lock().unwrap();
			process_command(&mut s, c);
		    }
		}
		_ = channels.jack_tick.recv() => {
		    let mut s = s_clone2.lock().unwrap();
		    self.process_sequence_data(&mut channels, &mut s);
		}
	    }
	}
    }
    fn process_sequence_data(
        &self,
        channels: &mut TrackAudioChannels,
	state: &mut TrackAudioState,
    ) {
	if state.playing {
	    let mut buf = Vec::new();
	    let mut first = true;

	    let n = state.sequences.len();
    	    let channels = RefCell::new(channels);

    	    //todo remove me
//	    let n = 1;
//    	    let mut wave = sine_wave_generator(&440f32, 128, 48000);
	    for i in 0..n {
		/*
		//todo remove me
		for _ in 0..128 {
		    let x = wave.pop().unwrap();
		    buf.push((x, x));
    	    }
		*/
		let mut channels_ref = channels.borrow_mut();
		if let Some(mut seq) = state.sequences.get(i) {
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
		}
		for tup in buf.iter() {
		    channels_ref.output.send(*tup);
		}

	    }
	}
    }
}


