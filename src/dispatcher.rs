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
use tokio::sync::oneshot;
use std::collections::VecDeque;
use std::collections::HashMap;

pub struct Dispatcher {
    jack_command_tx: mpsc::Sender<JackioCommand>,
    midi_in_vec: Vec<Receiver<OwnedMidi>>,
    midi_out_vec: Vec<OwnedMidi>,
    nsm: nsm::Client,
    path: Option<String>,
    audio_sequences: Vec<AudioSequenceCommander>,
    scenes: Vec<Scene>,
    jsfc: JackSyncFanoutCommander,
    audio_in_switch_commander: AudioInSwitchCommander,
    track_combiners: Vec<TrackAudioCombinerCommander>,
    framerate: usize,
    beats_per_bar: usize,
    last_frame: usize,
    current_scene: usize,
    recording_sequences: Vec<usize>,
    playing_sequences: Vec<usize>,
    newest_sequences: Vec<usize>,
    jsf_rx: mpsc::Receiver<JackSyncFanoutMessage>
}

impl Dispatcher {
    pub async fn new (
	jack_command_tx: mpsc::Sender<JackioCommand>,
        midi_in_vec: Vec<Receiver<OwnedMidi>>,
        midi_out_vec: Vec<OwnedMidi>,
	jack_client_addr: usize,
	tick_rx: mpsc::Receiver<()>,
        mut audio_out: VecDeque<Sender<(f32, f32)>>,
        audio_in_vec: Vec<Receiver<(f32, f32)>>,
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
	
	let mut jsfc = JackSyncFanoutCommander::new(tick_rx, jack_client_addr);
	//dispatcher's jsf
	let (jsf_tx, mut jsf_rx) = mpsc::channel(1);
	jsfc.send_command(JackSyncFanoutCommand::NewRecipient{ sender: jsf_tx }).await;
	
	let (ais_jsf_tx, mut ais_jsf_rx) = mpsc::channel(1);
	let mut audio_in_switch_commander = AudioInSwitchCommander::new(
	    audio_in_vec,
	    ais_jsf_rx
	);
	
	jsfc.send_command(JackSyncFanoutCommand::NewRecipient{ sender: ais_jsf_tx }).await;

	let mut track_combiners = Vec::new();
	for i in 0..AUDIO_TRACK_COUNT {
	    let (tx, rx) = mpsc::channel(1);
	    jsfc.send_command(JackSyncFanoutCommand::NewRecipient{ sender: tx }).await;
	    if let Some(chan) = audio_out.pop_front() {
		let t = TrackAudioCombinerCommander::new(chan, rx);
		track_combiners.push(t);
	    }
	}
	let mut framerate = 0;
	let mut beats_per_bar = 0;
	let mut last_frame = 0;
	let mut current_scene = 1;
	
	let mut recording_sequences = Vec::<usize>::new();
	let mut playing_sequences = Vec::<usize>::new();
	let mut newest_sequences = Vec::<usize>::new();

	Dispatcher {
	    jack_command_tx,
            midi_in_vec, 
            midi_out_vec,
	    nsm,
	    path,
	    audio_sequences,
	    scenes,
	    jsfc,
	    audio_in_switch_commander,
	    track_combiners,
	    framerate,
	    beats_per_bar,
	    last_frame,
	    current_scene,
	    recording_sequences,
	    playing_sequences,
	    newest_sequences,
	    jsf_rx
	}
    }
    pub async fn start(
	&mut self,
        command_midi_rx: mpsc::Receiver<OwnedMidi>,
    ) {
	let (command_req_tx, mut command_req_rx) = mpsc::channel(100);
	let non_sync_command_channel = command_req_tx.clone();
	let bar_boundary_command_channel = command_req_tx.clone();
	let (command_manager_out_tx, mut command_manager_out_rx) = mpsc::channel(100);
	let command_manager = CommandManager::new(command_manager_out_tx);
	command_manager.start(
	    command_midi_rx,
	    command_req_rx,
	);

	let mut sync_message_received = false;
	let mut load_request_ready = false;
	
	loop {
	    tokio::select!{
		jsf_msg_o = self.jsf_rx.recv() => {
		    if let Some(jsf_msg) = jsf_msg_o {
			sync_message_received = true;
			self.framerate = jsf_msg.framerate;
			self.beats_per_bar = jsf_msg.beats_per_bar;
			self.last_frame = jsf_msg.pos_frame;
			if jsf_msg.beat_this_cycle && jsf_msg.beat == 1 {
			    bar_boundary_command_channel.send(CommandManagerRequest::BarBoundary).await;
			}
			if load_request_ready {
			    self.process_load_request().await;
			    load_request_ready = false;
			}
		    }
		}

		Some(nsmc_message) = self.nsm.rx.recv() => {
		    match nsmc_message {
			nsm::NSMClientMessage::Save => {
			    match &self.path {
				Some(p) => {
				    //send save messages to stuff
				    self.process_save_request().await;
				}
				None => {
				    println!("No path configured. Check NSM server.");
				}
			    }
			}
			nsm::NSMClientMessage::Open { path: p } => {
			    self.path = Some(p);
			    load_request_ready = true;
			}
		    }
		}
		Some(commands) = command_manager_out_rx.recv() => {
		    for c in &commands {
			match c {
			    CommandManagerMessage::Stop => {
				for id in &self.playing_sequences {
				    if let Some(seq) = self.audio_sequences.get(*id) {
					seq.send_command(SequenceCommand::Stop).await;
    					self.jack_command_tx.send(
					    JackioCommand::StopPlaying{track: seq.track}
    					).await;
				    }
				}
			    },
			    CommandManagerMessage::Undo => {
				dbg!(&self.newest_sequences);
				for id in &self.newest_sequences {
				    if let Some(seq) = self.audio_sequences.get(*id) {
					seq.send_command(SequenceCommand::Shutdown).await;
					let c = self.track_combiners.get(seq.track).unwrap();
					c.send_command(TrackAudioCommand::DelLastSeq).await;
					self.audio_sequences.remove(*id);
				    }
				    for scene_id in 0..self.scenes.len() {
					let mut scene = self.scenes.get_mut(scene_id).unwrap();
					scene.remove_sequence(*id);
					dbg!(&scene.sequences);
				    }
				}
				
				self.newest_sequences.clear();
				self.recording_sequences.clear();
				self.playing_sequences.clear();
				self.start_scene(self.current_scene).await;
			    },
			    CommandManagerMessage::Go { tracks: t, scenes: s } => {
				if sync_message_received {
				    if t.len() == 0 && s.len() == 0 {
					for seq_id in &self.recording_sequences {
					    dbg!(seq_id);
					    //stop recording and autoplay
					    let seq = self.audio_sequences.get(*seq_id).unwrap();
					    self.jack_command_tx.send(
						JackioCommand::StopRecording{track: seq.track}
					    ).await;
					    
					    seq.send_command(SequenceCommand::Play).await;
					    seq.send_command(SequenceCommand::StopRecord).await;

					    self.track_combiners
						.get(seq.track)
						.unwrap()
						.send_command(TrackAudioCommand::Play)
						.await;
					    self.playing_sequences.push(*seq_id);
					    
					    self.jack_command_tx.send(
						JackioCommand::StartPlaying{track: seq.track}
					    ).await;
					}
					self.recording_sequences.clear();
				    }

				    //create new sequences
				    for track in t {
					self.new_audio_sequence(*track).await;

					let seq_id = self.audio_sequences.len() - 1;
					self.audio_sequences
					    .get(seq_id)
					    .unwrap()
					    .send_command(SequenceCommand::StartRecord).await;
    					dbg!(seq_id);
    					&self.recording_sequences.push(seq_id);
					
					&self.newest_sequences.clear();
					&self.newest_sequences.push(seq_id);


					self.jack_command_tx.send(
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
				self.start_scene(*scene_id).await;
			    }
			}
		    }
		}
	    }//tokio::select
	}//loop
    }//start()
    async fn start_scene(&mut self, scene_id: usize) {
	println!("starting scene {}", scene_id);
	self.current_scene = scene_id;
	let scene = self.scenes.get(scene_id).unwrap();
	dbg!(&self.playing_sequences);
	//todo: don't stop sequences that are meant to keep playing.
	for seq_id in &self.playing_sequences {
	    let seq = self.audio_sequences.get(*seq_id).unwrap();
	    seq.send_command(SequenceCommand::Stop).await;
	    self.jack_command_tx.send(
		JackioCommand::StopPlaying{track: seq.track}
	    ).await;

	    self.track_combiners
		.get(seq.track)
		.unwrap()
		.send_command(TrackAudioCommand::Stop)
		.await;	    
	}
	self.playing_sequences.clear();
	dbg!(&scene.sequences);
	for seq_id in &scene.sequences {
	    let seq = self.audio_sequences.get(*seq_id).unwrap();
	    seq.send_command(SequenceCommand::Play).await;
	    self.jack_command_tx.send(
		JackioCommand::StartPlaying{track: seq.track}
	    ).await;
	    println!("starting sequence {} track {}", seq_id, seq.track);
	    self.playing_sequences.push(*seq_id);
	    self.track_combiners
		.get(seq.track)
		.unwrap()
		.send_command(TrackAudioCommand::Play)
		.await;	    
	}
    }
    
    async fn process_save_request(&mut self) {

	if let Some(p) = &self.path {
	    create_dir(p.to_string());
	let mut seq_maps = Vec::new();
	for seq in self.audio_sequences.iter_mut() {
		seq.send_command(SequenceCommand::Save { path: p.to_string() }).await;
	    seq.send_command(SequenceCommand::GetMeta).await;

	    match &seq.recv_reply().await {
		SequenceReply::Meta { track, beats, filename } => {
		    let item = SeqMeta {
			track: *track,
			beats: *beats,
			filename: (*filename.clone()).to_string()
		    };
		    seq_maps.push(item);
		}
		SequenceReply::Err { msg } => {
		    println!("Sequence replied with error: {}", msg);
		}
	    }

	}

	let mut scene_map = BTreeMap::new();
	for i in 0..(self.scenes.len() - 1) {
	    let mut sequence_names = Vec::new();
	    for idx in &self.scenes[i].sequences {
		match &seq_maps[*idx] {
		    SeqMeta { track, beats, filename } => {
			sequence_names.push((*filename.clone()).to_string());
		    }
		    _ => {}
		}
	    }
	    scene_map.insert(i, sequence_names);
	}
	
	println!("save {}/config.yaml", p.to_string());
	let mut config = File::create(format!("{}/config.yaml", p.to_string())).unwrap();

	let out = YamlConfig { scenes: scene_map, sequences: seq_maps };
	    config.write_all(serde_yaml::to_string(&out).unwrap().as_bytes());
	}
    }
    async fn process_load_request(&mut self) {

	let mut sequence_names = HashMap::new();
	let mut name_idx: usize = 0;
	if let Some(path) = self.path.clone() {
	    let config_yaml = format!("{}/config.yaml", path);
	    if let Ok(config) = File::open(config_yaml) {
		let config_data: YamlConfig = serde_yaml::from_reader(config).unwrap();
		for item in config_data.sequences {
		    let p = format!("{}/{}", path, item.filename);
		    self.new_audio_sequence(item.track).await;

		    sequence_names.insert(item.filename.clone().to_string(), name_idx);
		    name_idx = name_idx + 1;
		    if let Some(seq) = self.audio_sequences.get(self.audio_sequences.len() - 1){
			dbg!(&item);
			seq.send_command(SequenceCommand::Load {
			    path: p,
			    beats: item.beats
			}).await;			
		    }
		}

		for (i, seq_names) in config_data.scenes {
		    let mut seq_ids = Vec::new();
		    
		    for name in seq_names {
    			seq_ids.push(*sequence_names.get(&name).unwrap());
		    }
		    self.scenes[i] = Scene { sequences: seq_ids };
		}
	    }

	    self.start_scene(self.current_scene).await;

	} else {
	    println!("No path configured. Check NSM server");
	}
    }//process_load_request
    async fn new_audio_sequence(&mut self, track: usize) {
	let (j_tx, mut j_rx) = mpsc::channel(1);

	self.jsfc.send_command(
	    JackSyncFanoutCommand::NewRecipient{ sender: j_tx }
	).await;

	let (in_tx, in_rx) = unbounded();

	self.audio_in_switch_commander.send_command(
	    AudioInCommand::RerouteTrack {
		track: track,
		recipient: in_tx
	    }
	).await;

	//todo: scale channel size with frame size
	let (out_tx, out_rx) = unbounded();
	let mut combiner = self.track_combiners
	    .get_mut(track)
	    .unwrap();

	let _ = combiner.send_command(
    	    TrackAudioCommand::NewSeq {
    		channel: out_rx
    	    }
	).await;

	let new_seq_commander = AudioSequenceCommander::new(
	    track,
	    self.beats_per_bar,
	    self.last_frame,
	    self.framerate,
	    j_rx,
	    in_rx,
	    out_tx
	);
	self.audio_sequences.push(new_seq_commander);
    }//new_audio_sequence
}
