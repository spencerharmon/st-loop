//! GUI app state for st-loop.

use std::time::Instant;
use crate::constants::{AUDIO_TRACK_COUNT, SCENE_COUNT};
use jack::jack_sys as j;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SlotState {
	#[default]
	Empty,
	Recording,
	Playing,
	Stopped,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TransportView {
	pub state: j::jack_transport_state_t,
	pub bar: i32,
	pub beat: i32,
	pub tick: i32,
	pub beats_per_minute: f64,
	pub beats_per_bar: f32,
}

impl TransportView {
	pub fn state_label(&self) -> &'static str {
		match self.state {
			j::JackTransportStopped => "Stopped",
			j::JackTransportRolling => "Rolling",
			j::JackTransportStarting => "Starting",
			_ => "Unknown",
		}
	}
}

pub struct AppState {
	/// Address of the GUI's passive JACK client, used to query transport
	/// once per frame.
	pub client_addr: usize,
	pub transport: TransportView,
	/// 8×8 grid of slot state, indexed [scene][track].
	///
	/// TODO: wire this to the real `CommandManager` / `Dispatcher` state.
	/// Currently rendered as placeholder (all `Empty`) until a control
	/// channel is plumbed from the audio thread to the GUI.
	pub grid: [[SlotState; AUDIO_TRACK_COUNT]; SCENE_COUNT],
	pub last_refresh: Instant,
}

impl AppState {
	pub fn new(client_addr: usize) -> Self {
		Self {
			client_addr,
			transport: TransportView::default(),
			grid: Default::default(),
			last_refresh: Instant::now(),
		}
	}
}
