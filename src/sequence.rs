pub enum Sequence {
    AudioSequence,
}

pub trait TSequence <T> {
    fn record(&self);
    fn stop_recording (&self);
    fn process_position (&mut self) -> T; 
}
pub struct AudioSequence {
    pub track: usize,
    left: Vec<f32>,
    right: Vec<f32>,
    playhead: usize,
    length: usize
}

impl AudioSequence {
    pub fn new(track: usize) -> AudioSequence {
	let length = 10000;
	let left = Vec::new();
	let right = Vec::new();
	let playhead = 0;
	
	AudioSequence { track, left, right, playhead, length }
    }
    
    fn increment_playhead(&mut self) {
	if self.playhead == self.length {
	    self.playhead = 0;
	} else {
	    self.playhead = self.playhead + 1;
	}
    }
    pub fn process_record(&mut self, tup: (f32, f32)) {
	self.left.push(tup.0);
	self.right.push(tup.1);
    }
}

impl TSequence<(Vec<f32>, Vec<f32>)> for AudioSequence {
    fn record(&self) {
	();
    }
    fn stop_recording(&self) {
	();
    }
    fn process_position(&mut self) -> (Vec<f32>, Vec<f32>) {
	let mut ret_l = Vec::new();
	let mut ret_r = Vec::new();

	let last_frame = self.playhead + 128;
	for i in self.playhead..last_frame {
	    ret_l.push(*self.left.get(i).unwrap());
	    ret_r.push(*self.left.get(i).unwrap());
	    self.increment_playhead();
	}
	(ret_l, ret_r)
    }
}
