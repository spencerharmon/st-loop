#![feature(cell_leak,strict_provenance,drain_filter,get_mut_unchecked)]

mod jackio;
mod dispatcher;
mod track_audio;
mod audio_in_switch;
mod jack_sync_fanout;
mod command_manager;
mod scene;
mod track;
mod sequence;
mod midi_control;
mod constants;
mod nsm;
mod yaml_config;

use tokio;

#[tokio::main]
async fn main() {
    let io = jackio::JackIO::new();
    io.start().await;
    loop {
      continue
    }
}
