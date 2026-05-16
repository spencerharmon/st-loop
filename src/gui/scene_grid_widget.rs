//! 8×8 scene/track grid widget.
//!
//! Rows are scenes, columns are tracks. Each cell is a colored square
//! reflecting `SlotState`. Read-only in v1.

use eframe::egui::{self, Color32, Sense, Ui};
use crate::constants::{AUDIO_TRACK_COUNT, SCENE_COUNT};
use crate::gui::app_state::{AppState, SlotState};

const CELL: f32 = 36.0;
const GAP: f32 = 4.0;
const LABEL_COL: f32 = 56.0;

fn cell_color(state: SlotState, ui: &Ui) -> Color32 {
	match state {
		SlotState::Empty => ui.visuals().widgets.inactive.bg_fill,
		SlotState::Recording => Color32::from_rgb(0xff, 0x55, 0x55),
		SlotState::Playing => Color32::from_rgb(0x66, 0xff, 0x88),
		SlotState::Stopped => Color32::from_rgb(0xcc, 0xcc, 0x66),
	}
}

pub fn show(ui: &mut Ui, state: &AppState) {
	ui.heading("Scenes × Tracks");
	ui.add_space(4.0);

	// Column headers: track numbers.
	ui.horizontal(|ui| {
		ui.add_space(LABEL_COL);
		for t in 0..AUDIO_TRACK_COUNT {
			let (rect, _) = ui.allocate_exact_size(
				egui::vec2(CELL, 18.0),
				Sense::hover(),
			);
			ui.painter().text(
				rect.center(),
				egui::Align2::CENTER_CENTER,
				format!("T{}", t + 1),
				egui::FontId::monospace(12.0),
				ui.visuals().text_color(),
			);
			ui.add_space(GAP);
		}
	});

	for s in 0..SCENE_COUNT {
		ui.horizontal(|ui| {
			let (lab_rect, _) = ui.allocate_exact_size(
				egui::vec2(LABEL_COL, CELL),
				Sense::hover(),
			);
			ui.painter().text(
				lab_rect.center(),
				egui::Align2::CENTER_CENTER,
				format!("Scene {}", s + 1),
				egui::FontId::monospace(12.0),
				ui.visuals().text_color(),
			);

			for t in 0..AUDIO_TRACK_COUNT {
				let (rect, _) = ui.allocate_exact_size(
					egui::vec2(CELL, CELL),
					Sense::hover(),
				);
				let color = cell_color(state.grid[s][t], ui);
				ui.painter().rect_filled(rect, 4.0, color);
				ui.painter().rect_stroke(
					rect,
					4.0,
					egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.fg_stroke.color),
					egui::StrokeKind::Outside,
				);
				ui.add_space(GAP);
			}
		});
		ui.add_space(GAP);
	}
}
