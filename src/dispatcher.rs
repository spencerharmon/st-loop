use crossbeam_channel::*;
use crate::command_manager::CommandManager;
use crate::scene::Scene;
use crate::track::*;
use crate::constants::*;
use crate::sequence::*;
use crate::nsm;
use crate::yaml_config::*;
use crate::track_audio::*;
use crate::jack_sync_fanout::*;
use crate::track_audio::*;
use crate::jackio::*;
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
    command_rx: Receiver<OwnedMidi>,
    audio_in_vec: Vec<Receiver<(f32, f32)>>,
    midi_in_vec: Vec<Receiver<OwnedMidi>>,
    midi_out_vec: Vec<OwnedMidi>,
    scenes: Rc<RefCell<Vec<Scene>>>,
    audio_sequences: Rc<RefCell<Vec<RefCell<AudioSequence>>>>,
    nsm: nsm::Client
}

impl Dispatcher {
    pub fn new (
        command_rx: Receiver<OwnedMidi>,
        audio_in_vec: Vec<Receiver<(f32, f32)>>,
        midi_in_vec: Vec<Receiver<OwnedMidi>>,
        midi_out_vec: Vec<OwnedMidi>,
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

	//nsm client
	let nsm = nsm::Client::new();

	Dispatcher {
	    command_rx, 
            audio_in_vec,
            midi_in_vec, 
            midi_out_vec,
	    scenes,
	    audio_sequences,
	    nsm
	}
    }
    pub async fn start(
	mut self,
	tick_rx: mpsc::Receiver<()>,
        mut audio_out_vec: Vec<Sender<(f32, f32)>>,
	jack_command_tx: mpsc::Sender<JackioCommand>,
	jack_client_addr: usize,
    ) {
	let mut jsfc = JackSyncFanoutCommander::new(tick_rx, jack_client_addr);
	let mut track_combiners = Vec::new();
	let command_manager = CommandManager::new();
	command_manager.start();

	let (jsf_tx, mut jsf_rx) = mpsc::channel(1);
	jsfc = jsfc.send_command(JackSyncFanoutCommand::NewRecipient{ sender: jsf_tx }).await;

	for i in 0..AUDIO_TRACK_COUNT {
	    let (tx, rx) = mpsc::channel(1);
	    jsfc = jsfc.send_command(JackSyncFanoutCommand::NewRecipient{ sender: tx }).await;
	    let t = TrackAudioCombinerCommander::new(audio_out_vec.pop().unwrap(), rx);
	    //todo remove me


//	    let t = t.send_command(TrackAudioCommand::Play).await;

	    track_combiners.push(t);
	    
//	    jack_command_tx.send(JackioCommand::StartPlaying{track: i}).await;

	}
	let mut scene = 1;
	
	let mut path: String = "~/.config/st-tools/st-loop/".to_string(); 

	let recording_sequences = Rc::new(RefCell::new(Vec::<usize>::new()));
	let playing_sequences = Rc::new(RefCell::new(Vec::<usize>::new()));
	let newest_sequences = Rc::new(RefCell::new(Vec::<usize>::new()));


	loop {
	    let jsf_msg = jsf_rx.recv().await.unwrap();
	    if jsf_msg.beat_this_cycle && jsf_msg.beat == 1 {
		println!("cool!");
		//bar-aligned commands
		//go
	    }	    
	}
    }
 }
