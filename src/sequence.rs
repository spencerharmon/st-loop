pub enum Sequence {
    AudioSequence,
}

pub struct AudioSequence {
    pub track: usize,
    pub beats_per_bar: usize,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
    pub playhead: usize,
    pub length: usize,
    pub last_frame: usize,
    pub beat_counter: usize,
    pub n_beats: usize,
    pub startup_delay: bool,
    pub recording: bool
	
}

impl AudioSequence {
    pub fn new(track: usize, beats_per_bar: usize, last_frame: usize) -> AudioSequence {
	let length = 0;
	let left = Vec::new();
	let right = Vec::new();
	let playhead = 0;
	let beat_counter = 1;
	let n_beats = 0;
	let startup_delay = true;
	let recording = true;
	
	AudioSequence { track,
			beats_per_bar,
			left,
			right,
			playhead,
			length,
			last_frame,
			beat_counter,
			n_beats,
			startup_delay,
			recording
	}
    }

    pub fn process_record(&mut self, sample_pair: (f32, f32)) {
	if !self.recording || self.startup_delay {
	    return
	}
	// if self.startup_delay && beat {
	//     self.startup_delay = false;
	// } else {
	//     return
	// }
//	println!("sample: {}", sample_pair.0);
	self.left.push(sample_pair.0);
	self.right.push(sample_pair.1);
	self.length = self.length + 1;
    }

    pub fn observe_beat(&mut self) {
	if self.recording {
	    self.startup_delay = false;
	    self.n_beats = self.n_beats + 1;
	} else {

	    if self.beat_counter == self.n_beats {
		self.beat_counter = 1;
	    } else {
		self.beat_counter = self.beat_counter + 1;
	    }
	    let final_beat = self.beat_counter == self.n_beats;
	    if final_beat {
		self.playhead = 0;
	    }
	}
    }

    pub fn stop_recording(&mut self) {
	if !self.recording {
	    return
	}
	self.n_beats = (self.n_beats - (self.n_beats % self.beats_per_bar)) + self.beats_per_bar;
	self.recording = false;
	println!("stop recording. Beat length: {}", self.n_beats);
    }
    
    pub fn process_position(&mut self,
			    pos_frame: usize
    ) -> Option<Vec<(f32, f32)>> {
	let nframes = pos_frame - self.last_frame;
	if nframes == 0 {
	    return None
	}
	
	let mut ret = Vec::new();

	for i in 1..nframes + 1 {
	    //	    println!("{}", self.playhead);
	    if let Some(l) = self.left.get(self.playhead) {

		if let Some(r) = self.right.get(self.playhead) {
//		    println!("data {:?}", (*l, *r));
		    ret.push((*l, *r));

		}
	    } 
	    self.playhead = self.playhead + 1;
	}

	self.last_frame = pos_frame;
	if ret.len() == 0 {
	    None
	} else {
	    Some(ret)
	}
    }
}

