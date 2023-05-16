use crossbeam_channel::*;
use crate::scene::Scene;
use crate::track::*;
use crate::constants::*;
use crate::sequence::*;
use crate::nsm;
use crate::yaml_config::*;
use crate::track_audio::*;
use crate::tick_fanout::*;
use crate::track_audio::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::mem::MaybeUninit;
use std::collections::BTreeMap;
use std::fs::{File, create_dir};
use std::io::prelude::*;
use std::{thread, time};
use tokio::task;

pub struct Dispatcher {
    scenes: Rc<RefCell<Vec<Scene>>>,
    audio_sequences: Rc<RefCell<Vec<RefCell<AudioSequence>>>>,
    nsm: nsm::Client,
    tick_fanout: TickFanoutCommander,
    track_combiners: Vec<TrackAudioCombinerCommander>
}

impl Dispatcher {
    pub fn new (
	ps_rx: Receiver<()>,
        mut audio_out_vec: Vec<Sender<(f32, f32)>>,
    ) -> Dispatcher {

	//make scenes
	let scene_count = 8;
	let scenes = Rc::new(RefCell::new(Vec::new()));
	// plus 1 because the 0th scene is special empty scene
	for i in 0..scene_count + 1 {
	    scenes.borrow_mut().push(Scene{ sequences: Vec::new() });
	}

	//make sequences
	let audio_sequences = Rc::new(RefCell::new(Vec::new()));

	//st-sync client
//	let sync = st_sync::client::Client::new();

	//nsm client
	let nsm = nsm::Client::new();


	let mut tick_fanout = TickFanoutCommander::new(ps_rx);
	let mut track_combiners = Vec::new();
	for i in 0..AUDIO_TRACK_COUNT {
	    let (tick_tx, tick_rx) = bounded(1);
	    tick_fanout = tick_fanout.send_command(TickFanoutCommand::NewRecipient{ sender: tick_tx });
	    let t = TrackAudioCombinerCommander::new(audio_out_vec.pop().unwrap(), tick_rx);
	    //todo remove me

//	    let t = t.send_command(TrackAudioCommand::Play);

	    track_combiners.push(t);
	    
	    
//	    start_playing.send(i);

	}
	
	
	Dispatcher {
	    scenes,
	    audio_sequences,
	    nsm,
	    tick_fanout,
	    track_combiners
	}
    }
    pub async fn start(mut self) {
	let mut beat_this_cycle = false;

	let mut pos_frame = 0;
	let mut framerate = 48000;
	let mut last_frame = pos_frame;
	let mut beats_per_bar = 0;
	let mut beat = 0;
	let mut scene = 1;
	let mut governor_on = true;
	let mut path: String = "~/.config/st-tools/st-loop/".to_string(); 

	let recording_sequences = Rc::new(RefCell::new(Vec::<usize>::new()));
	let playing_sequences = Rc::new(RefCell::new(Vec::<usize>::new()));
	let newest_sequences = Rc::new(RefCell::new(Vec::<usize>::new()));



	loop {
	    thread::sleep(time::Duration::from_millis(10));
	}
    }

}
