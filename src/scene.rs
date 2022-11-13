use crate::sequence::Sequence;

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
}
