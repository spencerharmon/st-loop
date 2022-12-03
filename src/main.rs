#![feature(cell_leak,strict_provenance,drain_filter)]

mod jackio;
mod looper;
mod command_manager;
mod scene;
mod track;
mod sequence;
mod midi_control;
mod constants;

use tokio;

#[tokio::main]
async fn main() {
    let io = jackio::JackIO::new();
    io.start().await;
    loop {
	continue
    }
}
