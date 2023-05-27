use tokio::sync::mpsc::*;
use std::{thread, time};
use crate::scene::Scene;
use jack::RawMidi;
use st_lib::owned_midi::*;
use wmidi;
use crate::midi_control;
use crate::constants::*;

pub enum CommandManagerMessage {
    Go {
	tracks: Vec<usize>,
	scenes: Vec<usize>
    },
    Start { scene: usize },
    Stop,
    Undo
}

pub enum CommandManagerRequest {
    BarBoundary,
    Async
}

#[derive(Debug)]
pub struct CommandManager {
    rec_tracks_idx: Vec<usize>,
    rec_scenes_idx: Vec<usize>,
    play_scene_idx: usize,
    go: bool,
    trigger_scene: bool,
    undo: bool,
    stop: bool
}
impl CommandManager {
    pub fn new ()  -> CommandManager {
	let rec_tracks_idx = Vec::new();
	let rec_scenes_idx = Vec::new();
	let play_scene_idx = 0;
	
	CommandManager {
	    rec_tracks_idx,
	    rec_scenes_idx,
	    play_scene_idx,
	    go: false,
	    trigger_scene: false,
	    undo: false,
	    stop: false
	}
    }

    pub fn start(
	mut self,
	mut command_midi_rx: Receiver<OwnedMidi>,
	mut req_rx: Receiver<CommandManagerRequest>,
	reply_tx: Sender<Vec<CommandManagerMessage>>
    ){
        tokio::task::spawn(async move {
	    self.thread(
		command_midi_rx,
		req_rx,
		reply_tx
	    ).await;
	});
    }
    
    async fn thread(
	&mut self,
	mut command_midi_rx: Receiver<OwnedMidi>,
	mut req_rx: Receiver<CommandManagerRequest>,
	reply_tx: Sender<Vec<CommandManagerMessage>>
    ){
	loop {
	    tokio::select!{
		midi = command_midi_rx.recv() => {
		    self.process_midi(midi.unwrap());
		}
		req = req_rx.recv() => {
		    if let Some(msg) = self.process_request(req.unwrap()){
			reply_tx.send(msg).await;
		    }
		}
	    }
	}
    }

    pub fn process_request(
	&mut self,
	req: CommandManagerRequest
    ) -> Option<Vec<CommandManagerMessage>>{
	let mut ret = Vec::new();
	if let req = CommandManagerRequest::BarBoundary {
	    
		if self.go {
		    let tracks = self.rec_tracks_idx.to_vec();
		    let scenes = self.rec_scenes_idx.to_vec();
		    ret.push(
			CommandManagerMessage::Go {
			    tracks,
			    scenes
			}
		    );
		    self.go = false;
		    self.rec_tracks_idx.clear();
		    self.rec_scenes_idx.clear();
		}
		if self.trigger_scene {
		    ret.push(
			CommandManagerMessage::Start {
			    scene: self.play_scene_idx
			}
		    );
		    self.trigger_scene = false;
		}
	}
	//command is otherwise CommandManagerRequest::Async
	if self.stop {
	    ret.push(CommandManagerMessage::Stop);
	}
	if self.undo {
	    ret.push(CommandManagerMessage::Undo);
	}

	if ret.len() == 0 {
	    return None
	}
	Some(ret)
    }
    
    pub fn process_midi(&mut self, om: OwnedMidi){
	println!("{:?}", wmidi::MidiMessage::from_bytes(&om.bytes));
	if let Ok(m) = wmidi::MidiMessage::from_bytes(&om.bytes) {
	    if m == midi_control::go() {
		self.go()
	    } else if m == midi_control::clear() {
		self.clear();
	    } else if m == midi_control::stop() {
		self.stop();
	    } else if m == midi_control::undo() {
		self.undo();
	    } else if m == midi_control::scene1() {
		self.scene(1);
	    } else if m == midi_control::scene2() {
		self.scene(2);
	    } else if m == midi_control::scene3() {
		self.scene(3);
	    } else if m == midi_control::scene4() {
		self.scene(4);
	    } else if m == midi_control::scene5() {
		self.scene(5);
	    } else if m == midi_control::scene6() {
		self.scene(6);
	    } else if m == midi_control::scene7() {
		self.scene(7);
	    } else if m == midi_control::scene8() {
		self.scene(8);
	    } else if m == midi_control::track0() {
		self.track(0);
	    } else if m == midi_control::track1() {
		self.track(1);
	    } else if m == midi_control::track2() {
		self.track(2);
	    } else if m == midi_control::track3() {
		self.track(3);
	    } else if m == midi_control::track4() {
		self.track(4);
	    } else if m == midi_control::track5() {
		self.track(5);
	    } else if m == midi_control::track6() {
		self.track(6);
	    } else if m == midi_control::track7() {
		self.track(7);
	    } // else if m == midi_control::track8() {
	    // 	self.track(8);
	    // } else if m == midi_control::track9() {
	    // 	self.track(9);
	    // } else if m == midi_control::track10() {
	    // 	self.track(10);
            // } else if m == midi_control::track11() {
            //  self.track(11);
            // } else if m == midi_control::track12() {
            //  self.track(12);
            // } else if m == midi_control::track13() {
            //  self.track(13);
            // } else if m == midi_control::track14() {
            //  self.track(14);
            // } else if m == midi_control::track15() {
            //  self.track(15);
            // } 
            println!("{:?}", self);
        }
    }

    fn go(&mut self) {
        println!("Go");
        self.go = true;
    }
    pub fn clear(&mut self) {
        println!("Clear");
        self.go = false;
        for _ in 0..self.rec_tracks_idx.len() {
            self.rec_tracks_idx.pop();
        }
        for _ in 0..self.rec_scenes_idx.len() {
            self.rec_scenes_idx.pop();
        }
        self.stop = false;
        self.undo = false;
    }
    
    fn stop(&mut self) {
        println!("Stop");
        self.stop = true;
        self.play_scene_idx = 0;
    }
    
    pub fn undo(&mut self) {
        println!("Undo");
        self.undo = true;
    }
    
    fn track(&mut self, n: usize){
        if !self.rec_tracks_idx.contains(&n){
            self.rec_tracks_idx.push(n);
	}
    }
    
    fn scene(&mut self, n: usize){
	if self.rec_tracks_idx.len() == 0 {
	    self.play_scene_idx = n;
	    self.trigger_scene = true;
	} else {
	    if !self.rec_scenes_idx.contains(&n){
		self.rec_scenes_idx.push(n);
	    }
	}
    }
}
