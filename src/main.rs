
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

mod yaml_config;

use tokio;

#[tokio::main]
async fn main() {
    console_subscriber::init();
    let io = jackio::JackIO::new();
    io.start().await;
}
