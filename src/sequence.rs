pub enum Sequence {
    AudioSequence,
}

pub struct AudioSequence {
    pub track: usize,
    left: Vec<f32>,
    right: Vec<f32>,
    playhead: usize,
    length: usize,
    last_frame: usize

}

impl AudioSequence {
    pub fn new(track: usize) -> AudioSequence {
	let length = 0;
	let left = Vec::new();
	let right = Vec::new();
	let playhead = 0;
	let last_frame = 0;
	
	AudioSequence { track, left, right, playhead, length, last_frame }
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
	self.length = self.length + 1;
	println!("{}", self.length);
    }

    pub fn stop_recording(&self) {
	();
    }
    pub fn process_position(&mut self,
			    pos_frame: usize,
    ) -> Vec<(f32, f32)> {
	let mut ret = Vec::new();
	
	let nframes = pos_frame - self.last_frame;

	if self.playhead + nframes > self.length {
	    for i in self.playhead..self.length {
		ret.push((*self.left.get(i).unwrap(), *self.left.get(i).unwrap()));
		self.increment_playhead();
	    }
	    for i in 0..((self.playhead+nframes) - self.length) {
		ret.push((*self.left.get(i).unwrap(), *self.left.get(i).unwrap()));
		self.increment_playhead();
	    }
	} else {
	    for i in self.playhead..(self.playhead + nframes + 1) {
		ret.push((*self.left.get(i).unwrap(), *self.left.get(i).unwrap()));
		self.increment_playhead();
	    }
	}
	self.last_frame = pos_frame;
	ret
    }
}
