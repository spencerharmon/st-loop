use crossbeam_channel::*;
use crate::command_manager::CommandManager;
use crate::scene::Scene;
use crate::track::*;
use crate::constants::*;
use crate::sequence::*;
use crate::nsm;
use crate::yaml_config::*;
use st_lib::owned_midi::*;
use std::rc::Rc;
use std::cell::RefCell;
use jack::jack_sys as j;
use std::mem::MaybeUninit;
use std::collections::BTreeMap;
use std::fs::{File, create_dir};
use std::io::prelude::*;


pub struct Looper {
    ps_rx: Receiver<()>,
    start_playing: Sender<usize>,
    stop_playing: Sender<usize>,
    start_recording: Sender<usize>,
    stop_recording: Sender<usize>,
    command_rx: Receiver<OwnedMidi>,
    audio_in_vec: Vec<Receiver<(f32, f32)>>,
    midi_in_vec: Vec<Receiver<OwnedMidi>>,
    audio_out_vec: Vec<(Sender<f32>, Sender<f32>)>,
    midi_out_vec: Vec<OwnedMidi>,
    jack_client_addr: usize,
    command_manager: CommandManager,
    scenes: Rc<RefCell<Vec<Scene>>>,
    audio_sequences: Rc<RefCell<Vec<RefCell<AudioSequence>>>>,
    sync: st_sync::client::Client,
    nsm: nsm::Client
	
}

impl Looper {
    pub fn new (
	ps_rx: Receiver<()>,
	start_playing: Sender<usize>,
	stop_playing: Sender<usize>,
	start_recording: Sender<usize>,
	stop_recording: Sender<usize>,
        command_rx: Receiver<OwnedMidi>,
        audio_in_vec: Vec<Receiver<(f32, f32)>>,
        midi_in_vec: Vec<Receiver<OwnedMidi>>,
        audio_out_vec: Vec<(Sender<f32>, Sender<f32>)>,
        midi_out_vec: Vec<OwnedMidi>,
	jack_client_addr: usize
    ) -> Looper {
	let command_manager = CommandManager::new();

	//make scenes
	let scene_count = 8;
//	let mut c = Vec::new();
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
	
	Looper {
	    ps_rx,
	    start_playing,
	    stop_playing,
	    start_recording,
	    stop_recording,
	    command_rx, 
            audio_in_vec,
            midi_in_vec, 
            audio_out_vec, 
            midi_out_vec,
	    jack_client_addr,
	    command_manager,
	    scenes,
	    audio_sequences,
	    sync,
	    nsm
	}
    }
    pub async fn start(mut self) {
	let recording_sequences = Rc::new(RefCell::new(Vec::<usize>::new()));
	let playing_sequences = Rc::new(RefCell::new(Vec::<usize>::new()));
	let newest_sequences = Rc::new(RefCell::new(Vec::<usize>::new()));

	let client_pointer: *const j::jack_client_t = std::ptr::from_exposed_addr(self.jack_client_addr);

	let mut next_beat_frame = (&self).get_first_beat_frame();
	
	let mut beat_this_cycle = false;


	let mut pos = MaybeUninit::uninit().as_mut_ptr();


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
	loop {
	    
	    let mut b_rec_seq = recording_sequences.borrow_mut();
	    let mut b_play_seq = playing_sequences.borrow_mut();
	    let mut b_newest_seqs = newest_sequences.borrow_mut();
	    let mut b_aud_seq = self.audio_sequences.borrow_mut();
	    let mut b_scenes = self.scenes.borrow_mut();
	    
	    if let Ok(p) = (&self).nsm.try_recv_open() {
		println!("{}", p);
		path = p;
		let config_yaml = format!("{}/config.yaml", path);
		if let Ok(config) = File::open(config_yaml) {
		    let config_data: YamlConfig = serde_yaml::from_reader(config).unwrap();

		    for item in config_data.sequences {
			let mut new_seq = AudioSequence::new(item.track, beats_per_bar, last_frame, framerate);
			new_seq.load(format!("{}/{}", path, item.filename));
			new_seq.n_beats = item.beats;
			new_seq.filename = item.filename;
			new_seq.reset_playhead();
			

			b_aud_seq.push(RefCell::new(new_seq));
		    }

		    for (i, seq_names) in config_data.scenes {
			let mut seq_ids = Vec::new();
			for name in seq_names {
			    for idx in 0..b_aud_seq.len() {
				if b_aud_seq[idx].borrow().filename == name {
				    seq_ids.push(idx);
				}
			    }
			}
			b_scenes[i] = Scene { sequences: seq_ids };
		    }
		}
	    }
	    if let Ok(_) = (&self).nsm.try_recv_save() {
		create_dir(&path);
		let mut seq_maps = Vec::new();
		for seq in b_aud_seq.iter() {
		    let b_seq =seq.borrow_mut();
		    b_seq.save(&path);
		    let item = SeqMeta {
			filename: (*b_seq).filename.to_string(),
			beats: (*b_seq).n_beats,
			track: (*b_seq).track,
		    };
		    seq_maps.push(item);
		}
		let mut scene_map = BTreeMap::new();
		let scenes = b_scenes.to_vec();
		for i in 0..scenes.len() {
		    let mut sequence_names = Vec::new();
		    for idx in &scenes[i].sequences {
			let s = b_aud_seq[*idx].borrow();
			sequence_names.push(s.filename.clone());
		    }
		    scene_map.insert(i, sequence_names);
		}
		
		self.save(scene_map, seq_maps, &path);
	    }
	    
	    unsafe {
		j::jack_transport_query(client_pointer, pos);

		pos_frame = (*pos).frame as usize;
		beats_per_bar = (*pos).beats_per_bar as usize;
		beat = (*pos).beat as usize;
	    }
	    let nframes = pos_frame - last_frame;

	    if pos_frame >= next_beat_frame {
//		println!("checking");
		if let Ok(frame) = (&self).sync.recv_next_beat_frame() {
		    next_beat_frame = frame as usize;
//		    println!("next beat frame: {}", next_beat_frame);
//		    println!("pos frame: {}", pos_frame);
		}
	    }
	    beat_this_cycle = false;
	    if (((last_frame < next_beat_frame) &&
		(next_beat_frame <= pos_frame))) ||
		last_frame == 0 {
		    beat_this_cycle = true;

    		}

	    if self.command_manager.undo {
		//clear recording vec
		b_rec_seq.clear();

		for seq_id in b_newest_seqs.iter() {

		    //remove data from sequences
		    let seq = b_aud_seq.get(*seq_id).unwrap();
		    seq.borrow_mut().clear();

		    //remove sequences from scenes
		    for sn in b_scenes.iter_mut() {
			sn.remove_sequence(*seq_id);
		    }
		    //remove from playing
		    println!("undo seq {}", *seq_id);
		    b_play_seq.drain_filter(|x| *x == *seq_id);
		}
		b_newest_seqs.clear();
	    }
	    if self.command_manager.stop {
		//stop occurs immediately
		for _ in 0..b_play_seq.len() {
		    let idx = b_play_seq.pop().unwrap();
		    let mut b_seq = b_aud_seq.get(idx).unwrap().borrow_mut();
		    b_seq.reset_playhead();

//		    self.stop_playing.try_send(b_seq.track);
		}
		self.command_manager.clear();
		scene = 0;
	    }

	    if self.command_manager.play_scene_idx != scene {
		//play scene occurs at start of next bar
		if beat_this_cycle && beat == 1 {
		    //remove all current tracks and reset them
		    for _ in 0..b_play_seq.len() {
			let idx = b_play_seq.pop().unwrap();
			let mut b_seq = b_aud_seq.get(idx).unwrap().borrow_mut();
			b_seq.reset_playhead();
			
//			self.stop_playing.try_send(b_seq.track);
		    }
		    scene = self.command_manager.play_scene_idx;
		    if let Some(scene) = b_scenes.get(scene) {
			for s in &scene.sequences {
			    b_play_seq.push(*s);

			    let b_seq = b_aud_seq.get(*s).unwrap().borrow_mut();
			    
			    self.start_playing.try_send(b_seq.track);
			}
		    }
		}
	    }
	    
	    //go command
            if self.command_manager.go {
                for i in 0..b_rec_seq.len() {
		    let s = b_rec_seq.get(i).unwrap();
		    let mut seq = b_aud_seq.get(*s).unwrap().borrow_mut();
		    let cur_scene = b_scenes.get(scene).unwrap();


		    // autoplay if sequence in cur scene
		    if cur_scene.sequences.contains(s){
			seq.start_playing(pos_frame);
			b_play_seq.push(*s);
			self.start_playing.try_send(seq.track);
		    }

		    // stop record after start play
		    if seq.recording {
			seq.stop_recording();
			
			//tell jackio to stop sending on this track
			self.stop_recording.try_send(seq.track);
			if seq.n_beats == 0 {
			    println!("Warning: beat length is 0. running undo command.");
			    self.command_manager.undo();
			}
		    }
		}

		for _ in 0..b_rec_seq.len() {
		    b_rec_seq.pop();
		}

		//create new sequences
		let mut once = true;
		for t_idx in self.command_manager.rec_tracks_idx.iter() {
		    self.start_recording.try_send(*t_idx);
		    let mut new_seq = AudioSequence::new(*t_idx, beats_per_bar, last_frame, framerate);
		    b_aud_seq.push(RefCell::new(new_seq));
		    let seq_idx = b_aud_seq.len() - 1;
		    b_aud_seq.get(seq_idx).unwrap().borrow_mut().set_id(seq_idx);
		    b_rec_seq.push(seq_idx);
		    for s_idx in self.command_manager.rec_scenes_idx.iter() {
			let mut scene = b_scenes.get_mut(*s_idx).unwrap();
			scene.add_sequence(seq_idx);
		    }
		    if once {
			b_newest_seqs.clear();
			once = false;
		    }
		    b_newest_seqs.push(seq_idx);
		}
		self.command_manager.clear();
            }
	    
	    
	    //recording sequences procedure
	    for s in b_rec_seq.iter() {
	        let seq = b_aud_seq.get(*s).unwrap();

	        let mut bseq = seq.borrow_mut();
	        let t = bseq.track;
		let in_stereo_tup_chan = self.audio_in_vec.get(t).unwrap();
		//		println!("try process record");
		//probably use unwrap and allow this to panic.
		let mut data = Vec::new();
		if in_stereo_tup_chan.len() >= nframes {
		    for i in 0..nframes {
			if let Ok(samples) = in_stereo_tup_chan.try_recv() {
			    data.push(samples);
			}
		    }
		}

		if beat_this_cycle {
		    bseq.observe_beat(beat);
		}
		for sample_pair in data {
		    bseq.process_record(sample_pair);
		}
	    }
	    
	    //playing sequences procedure
	    if let Ok(()) = self.ps_rx.try_recv(){
		governor_on = false;
	    }
	    if !governor_on || beat_this_cycle {
		let mut track_bytes = Vec::new();
		for _ in 0..AUDIO_TRACK_COUNT {
		    track_bytes.push(Vec::<(f32, f32)>::new());
		}

		for s in b_play_seq.iter() {
		    let seq = b_aud_seq.get(*s).unwrap();

		    let mut bseq = seq.borrow_mut();
		    if beat_this_cycle {
			bseq.observe_beat(beat);
		    }

		    let t = bseq.track;

		    //combine audio sequences in track
		    if let Some(seq_out) = bseq.process_position(nframes, pos_frame){

			let mut track_vec = track_bytes.get_mut(bseq.track).unwrap();

			if track_vec.len() == 0 {
			    *track_vec = seq_out;
			} else {
			    for i in 0..seq_out.len() {
				if let Some(tup) = track_vec.get_mut(i) {
				    tup.0 = tup.0 + seq_out.get(i).unwrap().0;
				    tup.1 = tup.1 + seq_out.get(i).unwrap().1;
				} else {
				    track_vec.push(*seq_out.get(i).unwrap());
				}
			    }
			}
		    }
		}

		for i in 0..AUDIO_TRACK_COUNT {
		    let track_vec = track_bytes.get_mut(i).unwrap();
		    let (chan_l, chan_r) = self.audio_out_vec.get(i).unwrap();
		    //todo: use fraction of sample rate
		    //set to lower number for bit crush distortion
		    if chan_l.len() > 1024 {
			governor_on = true;
		    }
		    // if beat_this_cycle {
		    //     println!("queue len: {}", chan_l.len());
		    // }
		    for (l, r) in track_vec.iter() {
    //		    println!("{}", *l);
			chan_l.try_send(*l);
			chan_r.try_send(*r);
		    }
		}
	    }

	    //process new commands
	    match self.command_rx.try_recv() {
		Ok(rm) => self.command_manager.process_midi(rm),
		Err(_) => ()
	    }
	    last_frame = pos_frame
	}//loop
    }//fn start
    fn get_first_beat_frame(&self) -> usize {
	loop {
	    if let Ok(frame) = self.sync.recv_next_beat_frame() {
		return frame as usize
	    }
	}
    }
    fn save(&self,
	    scene_map: BTreeMap<usize, Vec<String>>,
	    seq_maps: Vec<SeqMeta>,
	    path: &String) {
	println!("looper save {}", path);
	let mut config = File::create(format!("{}/config.yaml", path)).unwrap();

	let out = YamlConfig { scenes: scene_map, sequences: seq_maps };
	config.write_all(serde_yaml::to_string(&out).unwrap().as_bytes());
    }
}
