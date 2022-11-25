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
    pub n_beats: usize
}

impl AudioSequence {
    pub fn new(track: usize, beats_per_bar: usize) -> AudioSequence {
	let length = 0;
	let left = Vec::new();
	let right = Vec::new();
	let playhead = 0;
	let last_frame = 0;
	let beat_counter = 1;
	let n_beats = 0;
	
	AudioSequence { track,
			beats_per_bar,
			left,
			right,
			playhead,
			length,
			last_frame,
			beat_counter,
			n_beats
	}
    }

    pub fn process_record(&mut self, tup: (f32, f32), pos_frame: usize, next_beat_frame: usize) {
	self.left.push(tup.0);
	self.right.push(tup.1);
	self.length = self.length + 1;
	if ((self.last_frame < next_beat_frame) &&
	    (next_beat_frame <= pos_frame)) ||
	    self.last_frame == 0 {
			self.n_beats = self.n_beats + 1;
	    }
	self.last_frame = pos_frame;
    }

    pub fn stop_recording(&mut self) {
	self.n_beats = self.n_beats - (self.n_beats % self.beats_per_bar) + self.beats_per_bar;
	println!("stop recording. Beat length: {}", self.n_beats);
    }
    
    pub fn process_position(&mut self,
			    pos_frame: usize,
			    next_beat_frame: usize
    ) -> Option<Vec<(f32, f32)>> {
	let nframes = pos_frame - self.last_frame;

	if nframes == 0 {
	    return None
	}
	
	let mut ret = Vec::new();
	let mut beat_this_cycle = false;
	if ((self.last_frame < next_beat_frame) &&
	    (next_beat_frame <= pos_frame)) ||
	    self.last_frame == 0 {
		beat_this_cycle = true;
	}
	let final_beat = self.beat_counter == self.n_beats;
	let mut beat_frame = 0;
	if beat_this_cycle {
	    if self.last_frame == 0 {
		beat_frame = 1;
	    } else {
		beat_frame = next_beat_frame - self.last_frame;
		println!("beat frame: {}", beat_frame);
		println!("final beat: {}", final_beat);
		println!("n beats: {}", self.n_beats);
	    }
	}
	
	for i in 1..nframes + 1 {
	    //	    println!("{}", self.playhead);
	    if let Some(l) = self.left.get(self.playhead) {

		if let Some(r) = self.right.get(self.playhead) {
//		    println!("data {:?}", (*l, *r));
		    ret.push((*l * 0.8, *r * 0.8));

		}
	    } 
	    if beat_this_cycle && i == beat_frame {
		println!("beat");
		if self.beat_counter == self.n_beats {
		    self.beat_counter = 1;
		    self.playhead = self.playhead + 1;
		} else {
		    self.beat_counter = self.beat_counter + 1;
		    self.playhead = self.playhead + 1;
		}
		if final_beat {
		    self.playhead = 0;
		}
	    } else {
		self.playhead = self.playhead + 1;
	    }
	}
	
	self.last_frame = pos_frame;
	if ret.len() == 0 {
	    None
	} else {
	    Some(ret)
	}
    }
}

