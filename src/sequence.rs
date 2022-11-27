pub enum Sequence {
    AudioSequence,
}

pub struct AudioSequence {
    pub track: usize,
    pub beats_per_bar: usize,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
    pub playhead: usize,
    pub cycles_since_beat: usize,
    pub length: usize,
    pub last_frame: usize,
    pub beat_counter: usize,
    pub n_beats: usize,
    pub recording_delay: bool,
    pub playing_delay: bool,
    pub recording: bool,
    pub id: usize,
	
}

impl AudioSequence {
    pub fn new(track: usize, beats_per_bar: usize, last_frame: usize) -> AudioSequence {
	let length = 0;
	let left = Vec::new();
	let right = Vec::new();
	let playhead = 0;
	let cycles_since_beat = 0;
	let beat_counter = 1;
	let n_beats = 0;
	let recording_delay = true;
	let playing_delay = true;
	let recording = true;
	let id = 0;
	
	AudioSequence { track,
			beats_per_bar,
			left,
			right,
			playhead,
			cycles_since_beat,
			length,
			last_frame,
			beat_counter,
			n_beats,
			recording_delay,
			playing_delay,
			recording,
			id
	}
    }

    pub fn set_id(&mut self, id: usize) {
	self.id = id;
    }
    pub fn process_record(&mut self, sample_pair: (f32, f32)) {
	if !self.recording || self.recording_delay {
	    return
	}
	self.cycles_since_beat = self.cycles_since_beat + 1;

	self.left.push(sample_pair.0);
	self.right.push(sample_pair.1);
	self.length = self.length + 1;
	self.playhead = self.playhead + 1;
    }

    pub fn observe_beat(&mut self, beat: usize) {
	println!("id: {}", self.id);
	println!("beat: {}", beat);
	self.cycles_since_beat = 0;
	if self.recording {
	    if !self.recording_delay {
		self.n_beats = self.n_beats + 1;
	    }
	    if beat == 1 {
		self.recording_delay = false;
	    }
	} else {
	    if self.beat_counter == self.n_beats {
		println!("reset playhead");
		self.playhead = 0;
		self.beat_counter = 1;
	    } else {
	     	self.beat_counter = self.beat_counter + 1;
	    }
	    println!("playhead: {}", self.playhead);
	    println!("beat counter: {}", self.beat_counter);
	}
    }

    pub fn stop_recording(&mut self) {
	if !self.recording {
	    return
	}
	println!("n beats: {}", self.n_beats);
	// 1 beat wiggle room after bar start
	if self.n_beats % self.beats_per_bar == 0 {
	    self.beat_counter = 1;
	    //	    self.playhead = self.cycles_since_beat;
	    self.playhead = self.cycles_since_beat;
	    for _ in 0..self.cycles_since_beat {
		self.left.pop();
		self.right.pop();
	    }
	    //stop record goes after start play so we can override playing delay.
	    self.playing_delay = false;
	} else {
	    self.beat_counter = self.n_beats + 1;
	    self.n_beats = (self.n_beats - (self.n_beats % self.beats_per_bar)) + self.beats_per_bar;
	}
	
	// if self.beat_counter % self.beats_per_bar == 1 {
	//     self.n_beats = self.n_beats - 1;
	// } else {
	//     self.n_beats = (self.n_beats - (self.n_beats % self.beats_per_bar)) + self.beats_per_bar;
	// }
	self.recording = false;
	println!("stop recording. Beat length: {}", self.n_beats);
    }
    pub fn start_playing(&mut self, frame: usize) {
	self.last_frame = frame;
	self.playing_delay = true;
    }
    
    pub fn process_position(&mut self,
			    nframes: usize
    ) -> Option<Vec<(f32, f32)>> {
//	let nframes = pos_frame - self.last_frame;
	if nframes == 0 {
	    return None
	}
	if nframes > 128 {
	    println!("trouble");
	}
	if self.beat_counter == 1 {
	    if self.playing_delay {
		println!("playing delay off-----------------");
		self.playing_delay = false;
	    }
	}
	if self.playing_delay {
	    return None
	}

	let mut ret = Vec::new();

	for i in 1..nframes + 1 {
	    //	    println!("{}", self.playhead);
	    if let Some(l) = self.left.get(self.playhead) {

		if let Some(r) = self.right.get(self.playhead) {
//		    println!("data {:?}", (*l, *r));
		    ret.push((*l, *r));

		} else {
		    ret.push((0.0, 0.0));
		}
	    } else {
		ret.push((0.0, 0.0));
	    }

	    if self.playhead == 0 {
		println!("reset playhead worked");
	    }
	    self.playhead = self.playhead + 1;
	}

//	self.last_frame = pos_frame;
	Some(ret)
    }
}

