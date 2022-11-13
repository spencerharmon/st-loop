use crossbeam_channel::*;
use crate::sequence::*;



pub struct AudioTrack {
//    inputs: Receiver<(f32, f32)>,
//    outputs: Sender<(f32, f32)>
}


impl AudioTrack {
    pub fn send_output(&self, tup: (f32, f32)) {
	//self.outputs.send(tup);
	()
    }
    pub fn new_sequence(n: usize) -> AudioSequence {
	AudioSequence::new(n)
    }
}

