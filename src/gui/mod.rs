//! GUI glue for st-loop.
//!
//! Absorbs the eframe spike (`bin/gui_spike.rs`) into the production
//! tree. The GUI opens its own passive JACK client (`st-loop-gui`)
//! solely for `jack_transport_query`; the audio path is untouched.
//!
//! Wiring per-slot recording/playback state from `CommandManager` /
//! `Dispatcher` to `AppState::grid` is a future pass — see the TODO
//! on `AppState::grid`.

pub mod app_state;
pub mod scene_grid_widget;
pub mod transport_bar;

use eframe::egui;
use std::time::Duration;

use app_state::AppState;
use st_lib::{jack_ptr, jack_transport};

pub struct App {
	state: AppState,
}

impl App {
	pub fn new(state: AppState) -> Self {
		Self { state }
	}

	fn refresh(&mut self) {
		let snap = unsafe {
			let client = jack_ptr::recover_client(self.state.client_addr);
			jack_transport::query_transport(client)
		};
		self.state.transport = app_state::TransportView {
			state: snap.state,
			bar: snap.bar,
			beat: snap.beat,
			tick: snap.tick,
			beats_per_minute: snap.beats_per_minute,
			beats_per_bar: snap.beats_per_bar,
		};
		self.state.last_refresh = std::time::Instant::now();
	}
}

impl eframe::App for App {
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
		self.refresh();
		let ctx = ui.ctx().clone();

		egui::Panel::bottom("transport_bar").show_inside(ui, |ui| {
			transport_bar::show(ui, &self.state);
		});

		scene_grid_widget::show(ui, &self.state);

		ctx.request_repaint_after(Duration::from_millis(50));
	}
}

/// Open a passive JACK client for transport queries and launch the GUI
/// window. Blocks until the user closes the window.
pub fn run() -> Result<(), eframe::Error> {
	// Passive client: no ports, no process callback. Used solely so the
	// GUI can call `jack_transport_query` once per frame.
	let (client, _status) =
		jack::Client::new("st-loop-gui", jack::ClientOptions::NO_START_SERVER)
			.expect("failed to open passive JACK client for GUI");
	let client_addr = jack_ptr::expose_client(client.raw());

	let active_client = client
		.activate_async((), jack::ClosureProcessHandler::new(
			|_: &jack::Client, _ps: &jack::ProcessScope| -> jack::Control {
				jack::Control::Continue
			},
		))
		.expect("failed to activate GUI JACK client");
	// Leak it: the GUI client must live as long as the eframe window.
	std::mem::forget(active_client);

	let state = AppState::new(client_addr);

	let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default()
			.with_inner_size([720.0, 480.0])
			.with_title("st-loop"),
		..Default::default()
	};

	eframe::run_native(
		"st-loop",
		options,
		Box::new(move |cc| {
			cc.egui_ctx.set_visuals(egui::Visuals::dark());
			Ok(Box::new(App::new(state)))
		}),
	)
}
