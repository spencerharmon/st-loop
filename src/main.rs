#![feature(cell_leak,strict_provenance,drain_filter,get_mut_unchecked)]

mod dispatcher;
mod track_audio;
mod tick_fanout;
mod command_manager;
mod scene;
mod track;
mod sequence;
mod midi_control;
mod constants;
mod nsm;
mod yaml_config;
mod fake_jack;

use tokio;

#[tokio::main]
async fn main() {
    let io = fake_jack::FakeJack::new();
    io.start().await;
    loop {
      continue
    }
}
