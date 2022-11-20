use crossbeam_channel::*;
use crate::command_manager::CommandManager;
use crate::scene::Scene;
use crate::track::*;
use crate::constants::*;
use crate::sequence::*;
use st_lib::owned_midi::*;
use std::rc::Rc;
use std::cell::RefCell;
use jack::jack_sys as j;
use std::mem::MaybeUninit;


pub struct Looper {
    start_record: Sender<usize>,
    stop_record: Sender<usize>,
    command_rx: Receiver<OwnedMidi>,
    audio_in_vec: Vec<Receiver<(f32, f32)>>,
    midi_in_vec: Vec<Receiver<OwnedMidi>>,
    audio_out_vec: Vec<(Sender<f32>, Sender<f32>)>,
    midi_out_vec: Vec<OwnedMidi>,
    jack_client_addr: usize,
    command_manager: CommandManager,
    scenes: Rc<RefCell<Vec<Scene>>>,
    audio_sequences: Rc<RefCell<Vec<RefCell<AudioSequence>>>>,
}

impl Looper {
    pub fn new (
	start_record: Sender<usize>,
	stop_record: Sender<usize>,
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
	Looper {
	    start_record,
	    stop_record,
	    command_rx, 
            audio_in_vec,
            midi_in_vec, 
            audio_out_vec, 
            midi_out_vec,
	    jack_client_addr,
	    command_manager,
	    scenes,
	    audio_sequences
	}
    }
    pub async fn start(mut self) {
	let recording_sequences = Rc::new(RefCell::new(Vec::<usize>::new()));
	let playing_sequences = Rc::new(RefCell::new(Vec::<usize>::new()));

	let client_pointer: *const j::jack_client_t = std::ptr::from_exposed_addr(self.jack_client_addr);

	loop {
	    let mut b_rec_seq = recording_sequences.borrow_mut();
	    let mut b_play_seq = playing_sequences.borrow_mut();
	    let mut b_aud_seq = self.audio_sequences.borrow_mut();
	    let mut b_scenes = self.scenes.borrow_mut();
	    
	    let mut pos_frame = 0;
	    let mut pos = MaybeUninit::uninit().as_mut_ptr();
	    unsafe {
		j::jack_transport_query(client_pointer, pos);

		pos_frame = (*pos).frame as usize;
	    }
	    
	    match self.command_rx.try_recv() {
		Ok(rm) => self.command_manager.process_midi(rm),
		Err(_) => ()
	    }

	    //go command
            if self.command_manager.go {
		//first stop anything currently recording.
                for i in 0..b_rec_seq.len() {
		    let s = b_rec_seq.get(i).unwrap();
		    let mut seq = b_aud_seq.get(*s).unwrap().borrow_mut();
//		    seq.stop_record();

		    //set the last beat frame for newly-playing sequences
		    seq.last_frame = pos_frame;
		    
		    // always autoplay new sequences
		    b_play_seq.push(*s);
		    //and tell jackio to stop sending on this track
		    self.stop_record.send(*s);
		    println!("play new sequences: {:?}", b_play_seq);
		}
		for _ in 0..b_rec_seq.len() {
		    b_rec_seq.pop();
		}

		//create new sequences
                for t_idx in self.command_manager.rec_tracks_idx.iter() {
		    self.start_record.send(*t_idx);
		    let mut new_seq = AudioSequence::new(*t_idx);
		    b_aud_seq.push(RefCell::new(new_seq));
		    let seq_idx = b_aud_seq.len() - 1;
                    b_rec_seq.push(seq_idx);
                    for s_idx in self.command_manager.rec_scenes_idx.iter() {
			let mut scene = b_scenes.get_mut(*s_idx).unwrap();
			scene.add_sequence(seq_idx);
                    }
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
		match in_stereo_tup_chan.try_recv() {
		    Ok(data) => bseq.process_record(data),
		    Err(_) => ()
		}

	    }
	    //playing sequences procedure
	    let mut track_bytes = Vec::new();
	    for _ in 0..AUDIO_TRACK_COUNT {
		track_bytes.push(Vec::<(f32, f32)>::new());
	    }
	    
	    for s in b_play_seq.iter() {
		let seq = b_aud_seq.get(*s).unwrap();

		let mut bseq = seq.borrow_mut();

		let t = bseq.track;

		unsafe {
		    //combine audio sequences in track
		    let seq_out =  bseq.process_position(pos_frame);
		    
		    let mut track_vec = track_bytes.get_mut(bseq.track).unwrap();
		    if track_vec.len() == 0 {
			*track_vec = seq_out;
		    } else {
			for i in 0..seq_out.len() {
			    if let Some(tup) = track_vec.get_mut(i) {
				tup.0 = tup.0 + seq_out.get(i).unwrap().0;
				tup.1 = tup.1 + seq_out.get(i).unwrap().1;
			    }
//			    else {
//				track_vec.push(*seq_out.get(i).unwrap());
//			    }
			}
		    }
		}

	    }

	    for i in 0..AUDIO_TRACK_COUNT {
		let track_vec = track_bytes.get_mut(i).unwrap();
		let (chan_l, chan_r) = self.audio_out_vec.get(i).unwrap();
		for (l, r) in track_vec.iter() {
//		    println!("{}", *l);
		    chan_l.try_send(*l);
		    chan_r.try_send(*r);
		}
	    }
	}//loop
    }//fn start
}
