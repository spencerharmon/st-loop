use std::{thread, time};
use tokio::sync::mpsc::*;
use st_lib::{jack_ptr, jack_transport};
use crate::constants::*;

/// Per-cycle snapshot of timing facts, derived from JACK transport +
/// the st-sync beat-window protocol, broadcast to all downstream
/// consumers (dispatcher, sequence-audio threads, GUI).
#[derive(Copy, Clone, Debug)]
pub struct JackSyncFanoutMessage {
    /// Current JACK frame position (end of this process cycle).
    pub pos_frame: usize,
    /// JACK sample rate. Used by downstream code for resampling and
    /// WAV-file I/O.
    pub framerate: usize,
    /// Time-signature numerator. Read fresh from JACK transport on
    /// every cycle so meter changes propagate immediately.
    pub beats_per_bar: usize,
    /// JACK transport's beat-within-bar (1-indexed).
    pub beat: usize,
    /// Frames covered by this process cycle (pos_frame - prev_pos_frame).
    pub nframes: usize,
    /// Absolute JACK frame of the next published beat. Pulled from the
    /// st-sync window; `0` if the window doesn't yet cover a future
    /// beat (very early after startup).
    pub next_beat_frame: usize,
    /// Frames-per-beat for the just-completed beat, sourced from the
    /// st-sync window. `0` until the window has at least two beat
    /// frames. Authoritative — derived from the controller's own
    /// frame values, not re-computed locally from BPM + sample rate.
    pub frames_per_beat: usize,
    /// True iff a beat boundary fell within this process cycle.
    /// Detected by comparing `beat_position_at(last_frame).floor()` to
    /// `beat_position_at(pos_frame).floor()` via the st-sync window.
    pub beat_this_cycle: bool,
}


pub enum JackSyncFanoutCommand {
    NewRecipient { sender: Sender<JackSyncFanoutMessage> }
}

pub struct JackSyncFanoutCommander {
    tx: Sender<JackSyncFanoutCommand>,
}

impl JackSyncFanoutCommander {
    pub fn new(tick: Receiver<()>, jack_client_addr: usize) -> JackSyncFanoutCommander {
	let (command_tx, command_rx) = channel(1);
	let recipients = Vec::new();
	let channels = JackSyncFanoutChannels {
	    tick,
	    recipients
	};

	let fan = JackSyncFanout::new(jack_client_addr);
	unsafe {
            tokio::task::spawn(async move {
		fan.start(command_rx, channels, ).await;
	    });
	}

	JackSyncFanoutCommander {
	    tx: command_tx
	}
    }
    pub async fn send_command(&self, command: JackSyncFanoutCommand) {
	let _ = self.tx.send(command).await;
    }
}


#[derive(Debug)]
pub struct JackSyncFanoutChannels {
    tick: Receiver<()>,
    recipients: Vec<Sender<JackSyncFanoutMessage>>,
}

unsafe impl Send for JackSyncFanoutChannels {}


pub struct JackSyncFanout {
    jack_client_addr: usize,
    sync: st_sync::client::Client,
    last_frame: usize,
    /// Most recent beat position (in beats-past-window-start) we
    /// observed via `sync.beat_position_at`. Used to detect beat
    /// boundary crossings by floor() comparison across cycles.
    last_beat_pos: Option<f64>,
}

impl JackSyncFanout {
    pub fn new(jack_client_addr: usize) -> JackSyncFanout {
	let sync = st_sync::client::Client::new();
	// Wait for st-sync to publish enough beats that we can derive
	// frames_per_beat. Without that, downstream beat-detection has
	// no anchor.
	loop {
	    if sync.frames_per_beat().is_some() {
		break;
	    }
	    thread::sleep(time::Duration::from_millis(ASYNC_COMMAND_LATENCY));
	}

	JackSyncFanout {
	    jack_client_addr,
	    sync,
	    last_frame: 0,
	    last_beat_pos: None,
	}
    }

    async fn start(
	mut self,
	mut command_rx: Receiver<JackSyncFanoutCommand>,
	mut channels: JackSyncFanoutChannels
    ) {
	loop {
	    tokio::select! {
		command = command_rx.recv() => {
		    if let Some(c) = command {
			self.process_command(c, &mut channels);
		    }
		}
		_ = channels.tick.recv() => {
		    self.fanout_process(&mut channels);
		}
	    }
	}
    }

    fn process_command(
	&self,
	command: JackSyncFanoutCommand,
	channels: &mut JackSyncFanoutChannels
    ) {
	match command {
	    JackSyncFanoutCommand::NewRecipient { sender } => {
		channels.recipients.push(sender);
	    }
	}
    }

    fn fanout_process(
	&mut self,
	channels: &mut JackSyncFanoutChannels
    ) {
	let client_pointer = unsafe { jack_ptr::recover_client(self.jack_client_addr) };
	let snap = unsafe { jack_transport::query_transport(client_pointer) };
	let pos_frame = snap.frame as usize;

	// Derive timing facts from the st-sync window. These are the
	// values every client in the suite agrees on by construction.
	let frames_per_beat = self.sync.frames_per_beat().unwrap_or(0) as usize;
	let cur_beat_pos = self.sync.beat_position_at(pos_frame as u64)
	    // Fall back to extrapolation if the window doesn't yet cover
	    // this frame (rare; only at very edge cases).
	    .or_else(|| self.last_beat_pos.map(|prev| {
		if frames_per_beat == 0 { prev } else {
		    prev + (pos_frame.saturating_sub(self.last_frame)) as f64
			/ frames_per_beat as f64
		}
	    }));

	// Detect beat boundary crossing: floor(cur) != floor(prev).
	// On the very first cycle (no prev), we treat any non-empty
	// window as a beat — matches the prior code's "first cycle is
	// a beat" behavior.
	let beat_this_cycle = match (self.last_beat_pos, cur_beat_pos) {
	    (None, Some(_)) => true,
	    (Some(prev), Some(cur)) => cur.floor() as i64 != prev.floor() as i64,
	    _ => false,
	};

	// Next beat frame: walk the window for the first entry > pos_frame.
	let window = self.sync.snapshot().frames;
	let next_beat_frame = window.iter()
	    .find(|&&f| f as usize > pos_frame)
	    .copied()
	    .unwrap_or(0) as usize;

	let msg = JackSyncFanoutMessage {
	    pos_frame,
	    framerate: snap.frame_rate as usize,
	    beats_per_bar: snap.beats_per_bar as usize,
	    beat: snap.beat as usize,
	    nframes: pos_frame.saturating_sub(self.last_frame),
	    next_beat_frame,
	    frames_per_beat,
	    beat_this_cycle,
	};

	self.last_frame = pos_frame;
	self.last_beat_pos = cur_beat_pos;

	for recipient in &channels.recipients {
	    // try_send: drop the message if the recipient queue is full;
	    // we'd rather skip an update than block the fanout task.
	    let _ = recipient.try_send(msg);
	}
    }
}
