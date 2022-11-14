use crossbeam_channel::*;
use crate::command_manager::CommandManager;
use crate::scene::Scene;
use crate::track::*;
use crate::sequence::*;
use st_lib::owned_midi::*;
use std::rc::Rc;
use std::cell::RefCell;

pub struct Looper {
    command_rx: Receiver<OwnedMidi>,
    audio_in_vec: Vec<Receiver<(f32, f32)>>,
    midi_in_vec: Vec<Receiver<OwnedMidi>>,
    audio_out_vec: Vec<Sender<(f32, f32)>>,
    midi_out_vec: Vec<OwnedMidi>,
    command_manager: CommandManager,
    scenes: Rc<RefCell<Vec<Scene>>>,
    audio_sequences: Rc<RefCell<Vec<RefCell<AudioSequence>>>>,
}

impl Looper {
    pub fn new(
        command_rx: Receiver<OwnedMidi>,
        audio_in_vec: Vec<Receiver<(f32, f32)>>,
        midi_in_vec: Vec<Receiver<OwnedMidi>>,
        audio_out_vec: Vec<Sender<(f32, f32)>>,
        midi_out_vec: Vec<OwnedMidi>,
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
	    command_manager,
	    scenes,
	    audio_sequences
	}

    }
    pub async fn start(mut self) {
	let recording_sequences = Rc::new(RefCell::new(Vec::<usize>::new()));

	loop {
	    match self.command_rx.try_recv() {
		Ok(rm) => self.command_manager.process_midi(rm),
		Err(_) => ()
	    }

	    //go command
            if self.command_manager.go {
		let mut b_rec_seq = recording_sequences.borrow_mut();

		let mut b_aud_seq = self.audio_sequences.borrow_mut();
		let mut b_scenes = self.scenes.borrow_mut();

		//first stop anything currently recording.
                for s in b_rec_seq.iter() {
		    b_aud_seq.get(*s).unwrap().borrow_mut().stop_recording();
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

		//recording sequences procedure
		for s in b_rec_seq.iter() {
		    let bseqvec =self.audio_sequences.borrow_mut(); 
		    let seq = bseqvec.get(*s).unwrap();

		    let mut bseq = seq.borrow_mut();
		    let t = bseq.track;
		    bseq.process_record(
			self.audio_in_vec
			    .get(t)
			    .unwrap()
			    .recv()
			    .unwrap()
		    );
		}
                self.command_manager.clear();
            }
	}	    
    }
}
