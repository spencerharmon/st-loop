use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::select;
use tokio::sync::mpsc::{Receiver, Sender, UnboundedSender};
use std::cell::RefCell;
use crossbeam_channel;
use tokio::sync::RwLock;

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
        tokio::task::spawn(async move {
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
        let channels_rw = RwLock::new(channels);
        let mut channels_lock = channels_rw.write().await;
        
        loop {
            select! {
                command = command_rx.recv() => {
                    if let Some(c) = command {
                        self.process_command(&mut channels_lock, c);
                    }
                },
                _ = channels_lock.jack_tick.recv() => {
                    self.process_sequence_data(&mut channels_lock);
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
	//todo remove me
	let n = 1;
	let mut wave = sine_wave_generator(&440f32, 9999, 48000);
	let channels = RefCell::new(channels);
        for _ in 0..n {
	    //todo remove me
	    for _ in 0..9999 {
		let x = wave.pop().unwrap();
//	    dbg!(x);
		buf.push((x, x));
	    }
	    let mut channels_ref = channels.borrow_mut();
	    //todo remove if let; only for signal testing. ordinarily we can guarantee there's a seq at this point.
	    // let mut seq = channels_ref.sequences.pop().unwrap();
	    if let Some(mut seq) = channels_ref.sequences.pop() {
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
	    for (l, r) in buf.iter() {
		channels_ref.output.0.send(*l);
		channels_ref.output.1.send(*r);
	    }

	}
    }
}
