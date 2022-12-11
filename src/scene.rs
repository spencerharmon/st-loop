use crate::sequence::Sequence;

#[derive(Clone)]
pub struct Scene{
    pub sequences: Vec<usize>
}

impl Scene{
    pub fn new() -> Scene {
	let sequences = Vec::new();
	Scene { sequences }
    }
    pub fn add_sequence(& mut self, s: usize) {
	self.sequences.push(s);
	println!("sequence added. new length: {:?}", self.sequences.len());
    }
    pub fn remove_sequence(&mut self, seq: usize) {
	self.sequences.drain_filter(|x| *x == seq);
    }
}
