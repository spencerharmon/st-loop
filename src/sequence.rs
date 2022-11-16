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
	if self.playhead >= self.length {
	    self.playhead = 0;
	} else {
	    self.playhead = self.playhead + 1;
	}
    }
    pub fn process_record(&mut self, tup: (f32, f32)) {
	self.left.push(tup.0);
	self.right.push(tup.1);
	self.length = self.length + 1;
    }

    pub fn stop_recording(&self) {
	();
    }
    pub fn process_position(&mut self,
			    pos_frame: usize,
    ) -> Vec<(f32, f32)> {
	let mut ret = Vec::new();
	
	let nframes = pos_frame - self.last_frame;

	if nframes == 0 {
	    //transport paused
	    return ret
	}
	for i in 0..nframes {
	    //	    println!("{}", self.playhead);
	    //blech
	    match self.left.get(self.playhead) {
		Some(l) => {
		    match self.right.get(self.playhead) {

			Some(r) => {
//			    println!("data {:?}", (*l, *r));
			    ret.push((*l, *r));
			},
			_ => ()
		    }
		},
		_ => ()
	    }
	    // ret.push(
	    // 	(
	    // 	    *self.left.get(self.playhead).unwrap(),
	    // 	    *self.right.get(self.playhead).unwrap()
	    // 	)
	    // );
	    self.increment_playhead();
	}
	self.last_frame = pos_frame;
	ret
    }
}
