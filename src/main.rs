
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
mod gui;

use clap::Parser;

#[derive(Parser)]
struct Cli {
	/// Run headless (no GUI window).
	#[clap(long)]
	no_gui: bool,
}

fn main() {
	let cli = Cli::parse();

	console_subscriber::init();

	// Build the tokio runtime manually so we can `spawn` the JackIO
	// async task and still hand the main thread to eframe.
	let rt = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.expect("failed to build tokio runtime");

	let io = jackio::JackIO::new();
	rt.spawn(async move {
		io.start().await;
	});

	if cli.no_gui {
		// Headless: park the main thread; JackIO + dispatcher live on
		// the tokio runtime.
		loop {
			std::thread::park();
		}
	}

	if let Err(e) = gui::run() {
		eprintln!("eframe exited with error: {e}");
		std::process::exit(1);
	}
}
