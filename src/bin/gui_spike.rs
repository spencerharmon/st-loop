//! GUI integration spike for st-loop.
//!
//! Verifies the qwertysynth-style launch pattern composes with st-loop's
//! existing tokio + JACK + console-subscriber stack:
//!
//!   1. Synchronous `main()`.
//!   2. Build a multi-thread `tokio::runtime::Runtime` manually.
//!   3. `rt.spawn(...)` placeholder async tasks (in production: NSM,
//!      dispatcher, jackio).
//!   4. Call `eframe::run_native` on the main thread; it blocks.
//!
//! Communication between the GUI thread and async workers uses
//! `crossbeam_channel`, per the convention captured in plan.org.
//!
//! Run with:  cargo run --bin gui_spike -p st-loop
//!
//! eframe 0.34 specifics learned here (record for plan.org):
//!  - `App::ui(&mut Ui, &mut Frame)` is the primary trait method.
//!    `App::update(&Context, &mut Frame)` exists but is `#[deprecated]`.
//!  - `Panel::bottom("id")` (and `Panel::top`) — not the deprecated
//!    `TopBottomPanel`.
//!  - Panel `.show_inside(ui, ...)` — not `.show(ctx, ...)`.
//!  - `run_native`'s app-builder closure must return
//!    `Result<Box<dyn App>, Box<dyn Error + Send + Sync>>`, so wrap in `Ok`.
//!
//! This is a SPIKE — not part of st-loop's production binary. Delete (or
//! fold into a real `src/gui/` module) once the GUI implementation begins.

use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Stand-in for the real st-loop AppState. In production this will hold the
/// 8×8 scene grid, track levels, transport readout, etc.
#[derive(Default)]
struct AppState {
    /// Heartbeats received from the async worker so far.
    tick_count: u64,
    /// Last tick value seen (placeholder for next_beat_frame).
    last_tick: u64,
}

type SharedState = Arc<Mutex<AppState>>;

struct SpikeApp {
    state: SharedState,
    tick_rx: Receiver<u64>,
}

impl SpikeApp {
    fn new(state: SharedState, tick_rx: Receiver<u64>) -> Self {
        Self { state, tick_rx }
    }

    /// Drain channels into AppState. Called once per frame, holds the lock
    /// only for the duration of the drain. Matches qwertysynth's pattern.
    fn drain_ticks(&self) {
        let mut state = self.state.lock().unwrap();
        while let Ok(v) = self.tick_rx.try_recv() {
            state.tick_count += 1;
            state.last_tick = v;
        }
    }
}

impl eframe::App for SpikeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain_ticks();

        // The qwertysynth convention: draw directly into the central Ui
        // that eframe hands us; use TopBottomPanel for a status strip.
        let ctx = ui.ctx().clone();

        egui::Panel::bottom("info_bar").show_inside(ui, |ui| {
            let state = self.state.lock().unwrap();
            ui.horizontal(|ui| {
                ui.label(format!("ticks: {}", state.tick_count));
                ui.separator();
                ui.label(format!("last: {}", state.last_tick));
            });
        });

        ui.heading("st-loop GUI spike");
        ui.label("This window proves eframe coexists with st-loop's tokio runtime.");
        ui.label("Close the window to exit.");

        // Keep redrawing so the tick counter stays live.
        ctx.request_repaint_after(Duration::from_millis(50));
    }
}

/// Placeholder async worker — simulates the audio/sync thread pushing
/// ticks to the GUI via a crossbeam channel.
async fn fake_tick_producer(tx: Sender<u64>) {
    let mut n: u64 = 0;
    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;
        n = n.wrapping_add(1);
        if tx.send(n).is_err() {
            break; // GUI gone, exit cleanly
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    // (1) Synchronous main.
    let state: SharedState = Arc::new(Mutex::new(AppState::default()));
    let (tick_tx, tick_rx) = unbounded::<u64>();

    // (2) Manual multi-thread runtime — mirrors qwertysynth/main.rs.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    // (3) Spawn async workers. In production: NSM client, dispatcher, jackio.
    rt.spawn(fake_tick_producer(tick_tx));

    // (4) Hand the main thread to eframe.
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 500.0])
            .with_title("st-loop (spike)"),
        ..Default::default()
    };

    eframe::run_native(
        "st-loop (spike)",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(SpikeApp::new(state, tick_rx)))
        }),
    )
}
