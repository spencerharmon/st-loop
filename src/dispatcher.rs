use crossbeam_channel::*;
use crate::command_manager::*;
use crate::scene::Scene;
use crate::track::*;
use crate::constants::*;
use crate::sequence::*;
use crate::nsm;
use crate::yaml_config::*;
use crate::track_audio::*;
use crate::jack_sync_fanout::*;
use crate::track_audio::*;
use crate::audio_in_switch::*;
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
use std::collections::VecDeque;

pub struct Dispatcher {
    midi_in_vec: Vec<Receiver<OwnedMidi>>,
    midi_out_vec: Vec<OwnedMidi>,
    nsm: nsm::Client
}

impl Dispatcher {
    pub fn new (
        midi_in_vec: Vec<Receiver<OwnedMidi>>,
        midi_out_vec: Vec<OwnedMidi>,
    ) -> Dispatcher {
	//nsm client
	let nsm = nsm::Client::new();

	Dispatcher {
            midi_in_vec, 
            midi_out_vec,
	    nsm
	}
    }
    pub async fn start(
	mut self,
	tick_rx: mpsc::Receiver<()>,
        mut audio_out: VecDeque<Sender<(f32, f32)>>,
	jack_command_tx: mpsc::Sender<JackioCommand>,
	jack_client_addr: usize,
        command_midi_rx: mpsc::Receiver<OwnedMidi>,
        audio_in_vec: Vec<Receiver<(f32, f32)>>,
    ) {
	//make sequences
	let mut audio_sequences = Vec::<AudioSequenceCommander>::new();
	
	//make scenes
	let mut scenes = Vec::new();
	// plus 1 because the 0th scene is special empty scene
	for i in 0..SCENE_COUNT + 1 {
	    &scenes.push(Scene{ sequences: Vec::new() });
	}
	
	let mut jsfc = JackSyncFanoutCommander::new(tick_rx, jack_client_addr);
	//dispatcher's jsf
	let (jsf_tx, mut jsf_rx) = mpsc::channel(1);
	jsfc = jsfc.send_command(JackSyncFanoutCommand::NewRecipient{ sender: jsf_tx }).await;
	
	let (ais_jsf_tx, mut ais_jsf_rx) = mpsc::channel(1);
	let mut audio_in_switch_commander = AudioInSwitchCommander::new(
	    audio_in_vec,
	    ais_jsf_rx
	);
	jsfc = jsfc.send_command(JackSyncFanoutCommand::NewRecipient{ sender: ais_jsf_tx }).await;

	
	let (command_req_tx, mut command_req_rx) = mpsc::channel(100);
	let (command_reply_tx, mut command_reply_rx) = mpsc::channel(100);
	let command_manager = CommandManager::new();
	command_manager.start(
	    command_midi_rx,
	    command_req_rx,
	    command_reply_tx
	);


	let mut track_combiners = Vec::new();
	for i in 0..AUDIO_TRACK_COUNT {
	    let (tx, rx) = mpsc::channel(1);
	    jsfc = jsfc.send_command(JackSyncFanoutCommand::NewRecipient{ sender: tx }).await;
	    if let Some(chan) = audio_out.pop_front() {
		let t = TrackAudioCombinerCommander::new(chan, rx);
		track_combiners.push(t);
	    }
	}
	
	let mut current_scene = 1;
	let mut path: String = "~/.config/st-tools/st-loop/".to_string(); 

	let mut recording_sequences = Vec::<usize>::new();
	let mut playing_sequences = Vec::<usize>::new();
	let mut newest_sequences = Vec::<usize>::new();

	let mut sync_message_received = false;
	let mut framerate = 0;
	let mut beats_per_bar = 0;
	let mut last_frame = 0;
	loop {
	    //todo: this await should not be in the sync path. Spawn this into a thread and sleep for ASYNC_COMMAND_LATENCY 
	    command_req_tx.send(CommandManagerRequest::Async).await;
	    let mut commands = Vec::new();
	    tokio::select!{
		jsf_msg_o = jsf_rx.recv() => {
		    if let Some(jsf_msg) = jsf_msg_o {
			sync_message_received = true;
			framerate = jsf_msg.framerate;
			beats_per_bar = jsf_msg.beats_per_bar;
			last_frame = jsf_msg.pos_frame;
			if jsf_msg.beat_this_cycle && jsf_msg.beat == 1 {
			    println!("cool!");
			    command_req_tx.send(CommandManagerRequest::BarBoundary).await;
			}
		    }
		}

		opt = command_reply_rx.recv() => {
		    if let Some(c) = opt {
			commands = c;
		    }

		    for c in &commands {
			match c {
			    CommandManagerMessage::Stop => {
			    },
			    CommandManagerMessage::Undo => {
			    },
			    CommandManagerMessage::Go { tracks: t, scenes: s } => {
				if sync_message_received {
				    if t.len() == 0 && s.len() == 0 {
					for seq_id in &recording_sequences {
					    dbg!(seq_id);
					    //stop recording and autoplay
					    let seq = audio_sequences.get(*seq_id).unwrap();
					    seq.send_command(SequenceCommand::StopRecord).await;
					    jack_command_tx.send(
						JackioCommand::StopRecording{track: seq.track}
					    ).await;
					    
					    seq.send_command(SequenceCommand::Play).await;

					    playing_sequences.push(*seq_id);

					    
					    jack_command_tx.send(
						JackioCommand::StartPlaying{track: seq.track}
					    ).await;
					}
					recording_sequences.clear();
				    }

				    //create new sequences
				    for track in t {
					let (j_tx, mut j_rx) = mpsc::channel(1);

					jsfc = jsfc.send_command(
					    JackSyncFanoutCommand::NewRecipient{ sender: j_tx }
					).await;

					let (in_tx, in_rx) = unbounded();

					audio_in_switch_commander = audio_in_switch_commander.send_command(
					    AudioInCommand::RerouteTrack {
						track: *track,
						recipient: in_tx
					    }
					).await;

					let (out_tx, out_rx) = unbounded();
					let mut combiner = track_combiners
					    .get_mut(*track)
					    .unwrap();

					let _ = combiner.send_command(
						TrackAudioCommand::NewSeq {
						    channel: out_rx
						}
					).await;

					let new_seq_commander = AudioSequenceCommander::new(
					    *track,
					    beats_per_bar,
					    last_frame,
					    framerate,
					    j_rx,
					    in_rx,
					    out_tx
					);
					audio_sequences.push(new_seq_commander);

					let seq_id = audio_sequences.len() - 1;
    					dbg!(seq_id);
    					&recording_sequences.push(seq_id);
					&newest_sequences.push(seq_id);


					jack_command_tx.send(
					    JackioCommand::StartRecording{track: *track}
					).await;
				    }
				}
			    }, //go 
			    CommandManagerMessage::Start { scene: scene_id } => {
				current_scene = *scene_id;
				let scene = scenes.get(*scene_id).unwrap();
				for seq_id in &scene.sequences {
				    let seq = audio_sequences.get(*seq_id).unwrap();
    				    seq.send_command(SequenceCommand::Play).await;
    				    jack_command_tx.send(
    					JackioCommand::StartPlaying{track: seq.track}
				    ).await;
				}
			    }
			}
		    }
		}
	    }//tokio::select
	}//loop
    }//start()
 }
