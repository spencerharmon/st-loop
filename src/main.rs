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

//#[tokio::main]
fn main() {
    console_subscriber::init();
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
	    let io = jackio::JackIO::new();
	    io.start().await;
	    loop {
		continue
	    }
        })

}
