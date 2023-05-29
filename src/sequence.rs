use std::fs::File;
use std::time::{SystemTime, UNIX_EPOCH};
use rubato::{FftFixedIn, Resampler};
use std::cell::RefCell;
use crossbeam_channel::*;
use tokio::sync::mpsc;
use crate::jack_sync_fanout::*;

pub enum SequenceCommand {
    StartRecord,
    StopRecord,
    Play,
    Stop,
    Save { path: String },
    Load { path: String }
}

pub struct AudioSequenceCommander {
    tx: mpsc::Sender<SequenceCommand>
}

impl AudioSequenceCommander {
    pub fn new(
	track: usize,
	beats_per_bar: usize,
	last_frame: usize,
	framerate: usize,
	jack_sync_rx: mpsc::Receiver<JackSyncFanoutMessage>,
	audio_in: Receiver<(f32, f32)>,
	audio_out: Sender<(f32, f32)>
    ) -> AudioSequenceCommander {
	let (tx, mut rx) = mpsc::channel(1);

	let mut seq = AudioSequence::new(
	    track,
	    beats_per_bar,
	    last_frame,
	    framerate
	);

	tokio::spawn(async move {
	    seq.start(
		rx,
		jack_sync_rx,
		audio_in,
		audio_out
	    );
	});
	
	AudioSequenceCommander {
	    tx
	}
    }

    pub async fn send_command(&self, command: SequenceCommand) {
	self.tx.send(command).await;
    }
}

pub struct AudioSequence {
    playing: bool,
    pub recording: bool,
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
    pub id: usize,
    pub filename: String,
    framerate: usize
}

impl AudioSequence {
    pub fn new(
	track: usize,
	beats_per_bar: usize,
	last_frame: usize,
	framerate: usize,
    ) -> AudioSequence {
	let playing = false;
	let length = 0;
	let left = Vec::new();
	let right = Vec::new();
	let playhead = 0;
	let cycles_since_beat = 0;
	let beat_counter = 1;
	let n_beats = 0;
	let recording_delay = true;
	let playing_delay = false;
	let recording = true;
	let id = 0;
	let epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
	let filename = format!("{:?}-{:?}.wav", track, epoch);
	
	AudioSequence { playing,
			recording,
			track,
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
			id,
			filename,
			framerate
	}
    }
    async fn start(
	&mut self,
	mut command_rx: mpsc::Receiver<SequenceCommand>,
	mut jack_sync_rx: mpsc::Receiver<JackSyncFanoutMessage>,
	audio_in: Receiver<(f32, f32)>,
	audio_out: Sender<(f32, f32)>
    ){
	loop {
	    tokio::select! {
		cmd_o = command_rx.recv() => {
		}
		js_o = jack_sync_rx.recv() => {
		    if let Some(jack_sync_msg) = js_o {
			if self.playing {
			    if let Some(data) = self.process_position(
				jack_sync_msg.nframes,
				jack_sync_msg.pos_frame
			    ) {
				for tup in data { 
				    audio_out.send(tup);
				}
			    }
			} else if self.recording {
			    let tup = audio_in.recv().unwrap();
			    self.process_record(tup);
			}
		    }
		}
	    }
	}
    }

    pub fn set_id(&mut self, id: usize) {
	self.id = id;
    }
    pub fn clear(&mut self) {
	self.left.clear();
	self.right.clear();
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

    pub fn reset_playhead(&mut self) {
	self.playhead = 0;
	self.beat_counter = 1;
	self.playing_delay = false;
    }
    
    pub fn observe_beat(&mut self, beat: usize) {
	//todo: beat offset
	println!("id: {}", self.id);
	println!("beat: {}", beat);
        println!("playhead: {}", self.playhead);
        println!("beat counter: {}", self.beat_counter);
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
		self.reset_playhead();
	    } else {
	     	self.beat_counter = self.beat_counter + 1;
	    }
	}
    }

    pub fn stop_recording(&mut self) {
	if !self.recording {
	    return
	}
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
	
	self.recording = false;
	println!("stop recording. Beat length: {}", self.n_beats);
    }
    pub fn start_playing(&mut self, frame: usize) {
	self.last_frame = frame;
	self.playing_delay = true;
    }
    
    pub fn process_position(&mut self,
			    nframes: usize,
			    pos_frame: usize
    ) -> Option<Vec<(f32, f32)>> {
	if nframes == 0 {
	    return None
	}
	if pos_frame == self.last_frame {
//	    println!("yep");
	    return None
	}
//	if self.beat_counter == self.n_beats {
//	    if self.playing_delay {
//		println!("playing delay off-----------------");
//		self.playing_delay = false;
//	    }
//	}
	if self.playing_delay {
	    return None
	}

	let mut ret = Vec::new();

	for i in 1..nframes + 1 {
	    if let Some(l) = self.left.get(self.playhead) {

		if let Some(r) = self.right.get(self.playhead) {
		    ret.push((*l, *r));

		} 
	    } 

	    if self.playhead == 0 {
		println!("reset playhead worked");
	    }
	    self.playhead = self.playhead + 1;
	}

	self.last_frame = pos_frame;
	Some(ret)
    }
    pub fn save(&self, path: &String) {
	println!("sequence save {}", path);
	let full_path = format!("{}/{}", path, self.filename);
	if let Ok(_) = File::open(&full_path) {
	    println!("cowardly refusing to overwrite file");
	    return
	}

	let header = hound::WavSpec {
	    channels: 2,
	    sample_rate: self.framerate as u32,
	    bits_per_sample: 32,
	    sample_format: hound::SampleFormat::Float
	};

	let mut writer = hound::WavWriter::create(full_path, header).unwrap();

	for sample in interleave(&self.left, &self.right) {
	    writer.write_sample(sample);
	}

    }
    pub fn load(&mut self, file: String) {
	println!("load {}", file);
	let mut reader = hound::WavReader::open(file).unwrap();

	println!("file spec: {:?}", reader.spec());
	let bitness = reader.spec().bits_per_sample;

	let chunksize = 1024;
	if reader.spec().sample_rate as usize != self.framerate {
	    let mut resampler = FftFixedIn::<f64>::new(
		reader.spec().sample_rate as usize,
		self.framerate,
		chunksize,
		1024,
		reader.spec().channels as usize
	    ).unwrap();

	    println!("resample");
	    match reader.spec().sample_format {
		hound::SampleFormat::Float => {
		    println!("resample float");
    		    let mut samples = reader.samples::<f32>();
		    let chunk = RefCell::new(Vec::new());
		    let mut done = false;
		    loop {
			for i in 0..chunksize*2 {
			    if let Some(s) = samples.next() {
				chunk.borrow_mut().push(s.unwrap());
			    } else {
				chunk.borrow_mut().push(0.0);
				done = true;
			    }
			}
			let (chunk_l, chunk_r) = deinterleave(chunk.borrow_mut().to_vec());
			let dblvec = vec![vec_f32_to_f64(chunk_l), vec_f32_to_f64(chunk_r)];
			let out = resampler.process(&dblvec, None).unwrap();
			if let Some(l_chunk) = out.get(0) {
			    for s in l_chunk {
				self.left.push(*s as f32);
			    }
			}
			if let Some(r_chunk) = out.get(1) {
			    for s in r_chunk {
				self.right.push(*s as f32);
			    }
			}
			chunk.borrow_mut().clear();
			if done {
			    break
			}
		    }
		    
		},
		hound::SampleFormat::Int => {
		    println!("resample int");
    		    let mut samples = reader.samples::<i32>();
		    let mut chunk = RefCell::new(Vec::new());
		    let mut done = false;
		    loop {
			for i in 0..chunksize*2 {
			    if let Some(s) = samples.next() {
				let sample = (s.unwrap() as f32) / 2.0_f32.powf(bitness.into());
				chunk.borrow_mut().push(sample);
			    } else {
				chunk.borrow_mut().push(0.0);
				done = true;
			    }
			}
			let (chunk_l, chunk_r) = deinterleave(chunk.borrow_mut().to_vec());
			let dblvec = vec![vec_f32_to_f64(chunk_l), vec_f32_to_f64(chunk_r)];
			let out = resampler.process(&dblvec, None).unwrap();
			if let Some(l_chunk) = out.get(0) {
			    for s in l_chunk {
				self.left.push(*s as f32);
			    }
			}
			if let Some(r_chunk) = out.get(1) {
			    for s in r_chunk {
				self.right.push(*s as f32);
			    }
			}
			chunk.borrow_mut().clear();
			if done {
			    break
			}
		    }
		}
	    }
	} else {
	    println!("sample load native sample rate");
	    let mut data = Vec::new();
	    match reader.spec().sample_format {
		hound::SampleFormat::Float => {
		    for s in reader.samples::<f32>() {
			let sample = s.unwrap();
//			println!("{:?}", sample);
			data.push(sample);
		    }
		},
		hound::SampleFormat::Int => {
		    for s in reader.samples::<i32>() {
			let sample = (s.unwrap() as f32) / 2.0_f32.powf(bitness.into());
//			println!("{:?}", sample);
			data.push(sample);
		    }
		}
	    }
	    (self.left, self.right) = deinterleave(data);
	}
	self.recording = false;
    }
}

fn interleave(l: &Vec<f32>, r: &Vec<f32>) -> Vec<f32> {
    let mut ret = Vec::new();
    for i in 0..l.len() {
	ret.push(l[i]);
	ret.push(r[i]);
    }
    ret
}

fn vec_f32_to_f64(v: Vec<f32>) -> Vec<f64> {
    let mut ret = Vec::new();
    for s in v {
	ret.push(s as f64);
    }
    ret
}

fn deinterleave(v: Vec<f32>) -> (Vec<f32>, Vec<f32>) {
    let mut l = Vec::new();
    let mut r = Vec::new();
    
    for i in 0..v.len() {
	if i % 2 == 0 {
	    l.push(v[i]);
	} else if i % 2 == 1 {
	    r.push(v[i]);
	}
    }
    (l, r)
}
