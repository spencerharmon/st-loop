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
    pub beat: usize,
    pub recording_delay: bool,
    pub playing_delay: bool,
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
	let beat = 0;
	let recording_delay = true;
	let playing_delay = true;
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
			beat,
			recording_delay,
			playing_delay,
			recording
	}
    }

    pub fn process_record(&mut self, sample_pair: (f32, f32)) {
	if !self.recording || self.recording_delay {
	    return
	}
	// if self.recording_delay && beat {
	//     self.recording_delay = false;
	// } else {
	//     return
	// }
//	println!("sample: {}", sample_pair.0);
	self.left.push(sample_pair.0);
	self.right.push(sample_pair.1);
	self.length = self.length + 1;
	self.playhead = self.playhead + 1;
    }

    pub fn observe_beat(&mut self, beat: usize) {
	println!("beat: {}", beat);
	if self.recording {
	    self.beat = beat;
	    if beat == 1 {
		self.recording_delay = false;
	    }
	    if !self.recording_delay {
		self.n_beats = self.n_beats + 1;
	    }
	} else {
	    self.beat = beat;
	    if self.beat_counter == self.n_beats {
		self.playhead = 0;
		self.beat_counter = 1;
	    } else {
	     	self.beat_counter = self.beat_counter + 1;
	    }
	    println!("beat counter: {}", self.beat_counter);
	}
    }

    pub fn stop_recording(&mut self) {
	if !self.recording {
	    return
	}
	self.beat_counter = self.n_beats;
	self.n_beats = (self.n_beats - (self.n_beats % self.beats_per_bar)) + self.beats_per_bar;
	self.recording = false;
	println!("stop recording. Beat length: {}", self.n_beats);
    }
    pub fn start_playing(&mut self, frame: usize) {
	self.last_frame = frame;
	self.playing_delay = true;
    }
    
    pub fn process_position(&mut self,
			    pos_frame: usize
    ) -> Option<Vec<(f32, f32)>> {
	let nframes = pos_frame - self.last_frame;
	if nframes == 0 {
	    return None
	}
	if self.beat_counter == 1 {
//	    println!("playing delay off-----------------");
	    self.playing_delay = false;
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

