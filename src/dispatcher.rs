use crossbeam_channel::*;
use crate::command_manager::CommandManager;
use crate::scene::Scene;
use crate::track::*;
use crate::constants::*;
use crate::sequence::*;
use crate::nsm;
use crate::yaml_config::*;
use crate::track_audio::*;
use crate::tick_fanout::*;
use crate::track_audio::*;
use st_lib::owned_midi::*;
use std::rc::Rc;
use std::cell::RefCell;
use jack::jack_sys as j;
use std::mem::MaybeUninit;
use std::collections::BTreeMap;
use std::fs::{File, create_dir};
use std::io::prelude::*;
use std::{thread, time};
use tokio::task;
use tokio::sync::mpsc;

pub struct Dispatcher {
    stop_playing: Sender<usize>,
    start_recording: Sender<usize>,
    stop_recording: Sender<usize>,
    command_rx: Receiver<OwnedMidi>,
    audio_in_vec: Vec<Receiver<(f32, f32)>>,
    midi_in_vec: Vec<Receiver<OwnedMidi>>,
    midi_out_vec: Vec<OwnedMidi>,
    jack_client_addr: usize,
    command_manager: CommandManager,
    scenes: Rc<RefCell<Vec<Scene>>>,
    audio_sequences: Rc<RefCell<Vec<RefCell<AudioSequence>>>>,
    sync: st_sync::client::Client,
    nsm: nsm::Client
}

impl Dispatcher {
    pub fn new (
	stop_playing: Sender<usize>,
	start_recording: Sender<usize>,
	stop_recording: Sender<usize>,
        command_rx: Receiver<OwnedMidi>,
        audio_in_vec: Vec<Receiver<(f32, f32)>>,
        midi_in_vec: Vec<Receiver<OwnedMidi>>,
        midi_out_vec: Vec<OwnedMidi>,
	jack_client_addr: usize
    ) -> Dispatcher {
	let command_manager = CommandManager::new();

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
	let sync = st_sync::client::Client::new();

	//nsm client
	let nsm = nsm::Client::new();


	
	Dispatcher {
	    stop_playing,
	    start_recording,
	    stop_recording,
	    command_rx, 
            audio_in_vec,
            midi_in_vec, 
            midi_out_vec,
	    jack_client_addr,
	    command_manager,
	    scenes,
	    audio_sequences,
	    sync,
	    nsm
	}
    }
    pub async fn start(
	mut self,
	jack_tick_rx: mpsc::Receiver<()>,
        mut audio_out_vec: Vec<Sender<(f32, f32)>>,
	start_playing: Sender<usize>,
    ) {
	let mut tick_fanout = TickFanoutCommander::new(jack_tick_rx);
	let mut track_combiners = Vec::new();

	for i in 0..AUDIO_TRACK_COUNT {
	    let (tick_tx, tick_rx) = mpsc::channel(1);
	    tick_fanout = tick_fanout.send_command(TickFanoutCommand::NewRecipient{ sender: tick_tx }).await;
	    let t = TrackAudioCombinerCommander::new(audio_out_vec.pop().unwrap(), tick_rx);
	    //todo remove me


	    let t = t.send_command(TrackAudioCommand::Play).await;

	    track_combiners.push(t);
	    
	    start_playing.send(i);

	}
	let mut next_beat_frame = (&self).get_first_beat_frame();
	
	let mut beat_this_cycle = false;


	let mut pos = MaybeUninit::uninit().as_mut_ptr();

	let client_pointer: *const j::jack_client_t = std::ptr::from_exposed_addr(self.jack_client_addr);

	let mut pos_frame = 0;
	let mut framerate = 48000;
	unsafe {
	    j::jack_transport_query(client_pointer, pos);
	    pos_frame = (*pos).frame as usize;
	    framerate = (*pos).frame_rate as usize;
	}
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
	    /*
	    crossbeam::select! {
		recv(command_rx) -> midi_command {
		    if let Ok(c) = midi_command {
			self.command_manager.process_midi(c);
		    }
		}
	}
	    */
	    thread::sleep(time::Duration::from_millis(10));
	}
    }
    fn get_first_beat_frame(&self) -> usize {
	loop {
	    if let Ok(frame) = self.sync.try_recv_next_beat_frame() {
		return frame as usize
	    }
	    thread::sleep(time::Duration::from_millis(10));
	}
    }
}
