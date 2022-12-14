use std::collections::BTreeMap;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct YamlConfig {
    pub scenes: BTreeMap<usize, Vec<String>>,
    pub sequences: Vec<SeqMeta>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SeqMeta {
    pub filename: String,
    pub beats: usize,
    pub track: usize
}

