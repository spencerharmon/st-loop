//! Full-width transport status strip.

use eframe::egui::Ui;
use crate::gui::app_state::AppState;

pub fn show(ui: &mut Ui, state: &AppState) {
	ui.horizontal(|ui| {
		ui.label("st-loop");
		ui.separator();
		ui.label(state.transport.state_label());
		ui.separator();
		ui.label(format!(
			"{:>3} | {} | {:>4}",
			state.transport.bar, state.transport.beat, state.transport.tick
		));
		ui.separator();
		ui.label(format!("{:.2} BPM", state.transport.beats_per_minute));
		ui.separator();
		ui.label(format!("{}/{}", state.transport.beats_per_bar as u32, 4));
	});
}
