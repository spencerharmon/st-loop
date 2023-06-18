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
use std::collections::HashMap;

pub struct Dispatcher {
    midi_in_vec: Vec<Receiver<OwnedMidi>>,
    midi_out_vec: Vec<OwnedMidi>,
    nsm: nsm::Client,
    path: Option<String>,
    audio_sequences: Vec<AudioSequenceCommander>,
    scenes: Vec<Scene>
}

impl Dispatcher {
    pub fn new (
        midi_in_vec: Vec<Receiver<OwnedMidi>>,
        midi_out_vec: Vec<OwnedMidi>,
    ) -> Dispatcher {
	//nsm client
	let nsm = nsm::Client::new();
	let mut path: Option<String> = None;
	let mut audio_sequences = Vec::<AudioSequenceCommander>::new();

	//make scenes
	let mut scenes = Vec::new();
	// plus 1 because the 0th scene is special empty scene
	for i in 0..SCENE_COUNT + 1 {
	    &scenes.push(Scene{ sequences: Vec::new() });
	}
	Dispatcher {
            midi_in_vec, 
            midi_out_vec,
	    nsm,
	    path,
	    audio_sequences,
	    scenes,
	}
    }
    pub async fn start(
	&mut self,
	tick_rx: mpsc::Receiver<()>,
        mut audio_out: VecDeque<Sender<(f32, f32)>>,
	jack_command_tx: mpsc::Sender<JackioCommand>,
	jack_client_addr: usize,
        command_midi_rx: mpsc::Receiver<OwnedMidi>,
        audio_in_vec: Vec<Receiver<(f32, f32)>>,
    ) {
	//make sequences
	
	
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
	let non_sync_command_channel = command_req_tx.clone();
	let bar_boundary_command_channel = command_req_tx.clone();
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

	let mut recording_sequences = Vec::<usize>::new();
	let mut playing_sequences = Vec::<usize>::new();
	let mut newest_sequences = Vec::<usize>::new();

	let mut sync_message_received = false;
	let mut framerate = 0;
	let mut beats_per_bar = 0;
	let mut last_frame = 0;

	tokio::task::spawn(async move {
	    //todo we don't need this. command manager should send async command messages as soon as the midi message is received.
	    // this additional message passing wastes cpu cycles for no reason.
	    loop {
		non_sync_command_channel.send(CommandManagerRequest::Async).await;
		tokio::time::sleep(time::Duration::from_millis(ASYNC_COMMAND_LATENCY)).await;
	    }
	});
	loop {
	    let mut commands = Vec::new();
	    tokio::select!{
		jsf_msg_o = jsf_rx.recv() => {
		    if let Some(jsf_msg) = jsf_msg_o {
			sync_message_received = true;
			framerate = jsf_msg.framerate;
			beats_per_bar = jsf_msg.beats_per_bar;
			last_frame = jsf_msg.pos_frame;
			if jsf_msg.beat_this_cycle && jsf_msg.beat == 1 {
			    bar_boundary_command_channel.send(CommandManagerRequest::BarBoundary).await;
			}
		    }
		}

		Some(nsmc_message) = self.nsm.rx.recv() => {
		    match nsmc_message {
			nsm::NSMClientMessage::Save => {
			    match &self.path {
				Some(p) => {
				    //send save messages to stuff
				    self.process_save_request();
				}
				None => {
				    println!("No path configured. Check NSM server.");
				}
			    }
			}
			nsm::NSMClientMessage::Open { path: p } => {
			    self.path = Some(p);
			    //read the config
			    self.process_load_request();
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
				for id in &playing_sequences {
				    if let Some(seq) = self.audio_sequences.get(*id) {
					seq.send_command(SequenceCommand::Stop).await;
    					jack_command_tx.send(
					    JackioCommand::StopPlaying{track: seq.track}
    					).await;
				    }
				}
			    },
			    CommandManagerMessage::Undo => {
				for id in &newest_sequences {
				    if let Some(seq) = self.audio_sequences.get(*id) {
					seq.send_command(SequenceCommand::Shutdown).await;
					let c = track_combiners.get(seq.track).unwrap();
					c.send_command(TrackAudioCommand::DelLastSeq).await;
					self.audio_sequences.remove(*id);
				    }
				    for scene_id in 0..self.scenes.len() {
					let mut scene = self.scenes.get_mut(scene_id).unwrap();
					for i in 0..scene.sequences.len() {
					    let sid = scene.sequences.get(i).unwrap();
					    if sid == id {
						&scene.sequences.remove(i);
					    }
					}
				    }
				}
				
				playing_sequences.clear();
				newest_sequences.clear();
				recording_sequences.clear();
			    },
			    CommandManagerMessage::Go { tracks: t, scenes: s } => {
				if sync_message_received {
				    if t.len() == 0 && s.len() == 0 {
					for seq_id in &recording_sequences {
					    dbg!(seq_id);
					    //stop recording and autoplay
					    let seq = self.audio_sequences.get(*seq_id).unwrap();
					    jack_command_tx.send(
						JackioCommand::StopRecording{track: seq.track}
					    ).await;
					    
					    seq.send_command(SequenceCommand::Play).await;
					    seq.send_command(SequenceCommand::StopRecord).await;
					    
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
					self.audio_sequences.push(new_seq_commander);

					let seq_id = self.audio_sequences.len() - 1;
    					dbg!(seq_id);
    					&recording_sequences.push(seq_id);
					&newest_sequences.push(seq_id);


					jack_command_tx.send(
					    JackioCommand::StartRecording{track: *track}
					).await;

					//add sequence to scenes
					for scene_id in s {
					    let scene = self.scenes.get_mut(*scene_id).unwrap();
					    scene.sequences.push(seq_id);
					}
				    }
				}
			    }, //go 
			    CommandManagerMessage::Start { scene: scene_id } => {
				current_scene = *scene_id;
				let scene = self.scenes.get(*scene_id).unwrap();
				for seq_id in &playing_sequences {
				    let seq = self.audio_sequences.get(*seq_id).unwrap();
				    seq.send_command(SequenceCommand::Stop).await;
    				    jack_command_tx.send(
    					JackioCommand::StopPlaying{track: seq.track}
    				    ).await;
				}
				playing_sequences.clear();
				for seq_id in &scene.sequences {
				    let seq = self.audio_sequences.get(*seq_id).unwrap();
    				    seq.send_command(SequenceCommand::Play).await;
    				    jack_command_tx.send(
    					JackioCommand::StartPlaying{track: seq.track}
				    ).await;
				    playing_sequences.push(*seq_id);
				}
			    }
			}
		    }
		}
	    }//tokio::select
	}//loop
    }//start()
    fn process_save_request(&self) {
	for seq in &self.audio_sequences {
	    if let Some(p) = &self.path {
		seq.send_command(SequenceCommand::Save { path: p.to_string() });
	    }
	}
    }
    fn process_load_request(&mut self) {

	let mut sequence_names = HashMap::new();
	let mut name_idx: usize = 0;
	if let Some(path) = &self.path {
	    let config_yaml = format!("{}/config.yaml", path);
	    if let Ok(config) = File::open(config_yaml) {
		let config_data: YamlConfig = serde_yaml::from_reader(config).unwrap();
		for item in config_data.sequences {
			//todo actually make the sequence commander
    //		    let mut new_seq = AudioSequence::new(item.track, beats_per_bar, last_frame, framerate);
			let p = format!("{}/{}", path, item.filename);

		    sequence_names.insert(p, name_idx);
		    name_idx = name_idx + 1;
			/* todo send the load command via the new sequence commander
			seq.send_command(SequenceCommand::Load {
			    path: item.filename,
			    beats: item.beats
		    });
			*/
		}

		for (i, seq_names) in config_data.scenes {
		    let mut seq_ids = Vec::new();
		    for name in seq_names {
			for idx in 0..self.audio_sequences.len() {
    			    seq_ids.push(*sequence_names.get(&name).unwrap());
			}
		    }
		    self.scenes[i] = Scene { sequences: seq_ids };
		}
	    }

	} else {
	    println!("No path configured. Check NSM server");
	}
    }//process_load_request
}
