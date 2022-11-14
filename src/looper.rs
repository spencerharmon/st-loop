use crossbeam_channel::*;
use crate::command_manager::CommandManager;
use crate::scene::Scene;
use crate::track::*;
use crate::sequence::*;
use st_lib::owned_midi::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::cell::RefMut;
use jack::jack_sys as j;
use std::mem::MaybeUninit;

pub struct Looper {
    command_rx: Receiver<OwnedMidi>,
    audio_in_vec: Rc<RefCell<Vec<Receiver<(f32, f32)>>>>,
    midi_in_vec: Vec<Receiver<OwnedMidi>>,
    audio_out_vec: Rc<RefCell<Vec<Sender<(f32, f32)>>>>,
    midi_out_vec: Vec<OwnedMidi>,
    jack_client_addr: usize,
    command_manager: CommandManager,
    scenes: Rc<RefCell<Vec<Scene>>>,
    audio_sequences: Rc<RefCell<Vec<RefCell<AudioSequence>>>>,
}

impl Looper {
    pub fn new (
        command_rx: Receiver<OwnedMidi>,
        audio_in_vec: Rc<RefCell<Vec<Receiver<(f32, f32)>>>>,
        midi_in_vec: Vec<Receiver<OwnedMidi>>,
        audio_out_vec: Rc<RefCell<Vec<Sender<(f32, f32)>>>>,
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
	    let mut b_input = self.audio_in_vec.borrow_mut();
	    let mut b_output = self.audio_out_vec.borrow_mut();

	    match self.command_rx.try_recv() {
		Ok(rm) => self.command_manager.process_midi(rm),
		Err(_) => ()
	    }

	    //go command
            if self.command_manager.go {


		//first stop anything currently recording.
                for i in 0..b_rec_seq.len() {
		    let s = b_rec_seq.get(i).unwrap();
		    let seq = b_aud_seq.get(*s).unwrap().borrow_mut();
		    seq.stop_recording();
		    
		    // always autoplay new sequences
		    b_play_seq.push(*s);
		}
		for _ in 0..b_rec_seq.len() {
		    b_rec_seq.pop();
		}

		//create new sequences
                for t_idx in self.command_manager.rec_tracks_idx.iter() {
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
	        bseq.process_record(
	    	    b_input.get(t)
	    		.unwrap()
	    		.recv()
	    		.unwrap()
	        );
	    }
	    //playing sequences procedure
	    for s in b_play_seq.iter() {

		let seq = b_aud_seq.get(*s).unwrap();

		let mut bseq = seq.borrow_mut();

		let t = bseq.track;

		let mut pos = MaybeUninit::uninit().as_mut_ptr();
		unsafe {
		    j::jack_transport_query(client_pointer, pos);

		    bseq.process_position((*pos).frame as usize);
		}

	    }
	}	    
    }
}
