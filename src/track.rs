use crossbeam_channel::*;
use crate::sequence::*;
use std::rc::Rc;
use std::cell::RefCell;


pub struct AudioTrack{
    pub input_idx: usize,
    pub output_idx: usize
}


impl AudioTrack {
    pub fn send_output(&self, tup: (f32, f32)) {
	//self.outputs.send(tup);
	()
    }
    // pub fn new_sequence(n: usize) -> AudioSequence {
    // 	AudioSequence::new(n)
    // }
}

