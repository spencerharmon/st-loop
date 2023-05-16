use crate::constants::*;
use crate::dispatcher::*;
use std::cell::RefCell;
use crossbeam_channel::*;
use std::{thread, time};
use tokio;

pub struct FakeJack {}

impl FakeJack {
    pub fn new() -> FakeJack {
	FakeJack {}
    }
    pub async fn start(self) {
	let mut audio_out_tx_channels = Vec::<Sender<(f32, f32)>>::new();
	let ref_audio_out_tx_channels = RefCell::new(audio_out_tx_channels);

	for i in 0..AUDIO_TRACK_COUNT {
	    let mut b_audio_out_tx_channels = ref_audio_out_tx_channels.borrow_mut();
	    let (out_tx, out_rx) = unbounded();
	    b_audio_out_tx_channels.push(out_tx);
	}
	
	let (ps_tx, ps_rx) = bounded(1);
	
	tokio::task::spawn(async move {
	    loop {
		thread::sleep(time::Duration::from_millis(2));
		ps_tx.try_send(());
	    }

	});

	let audio_out_tx_channels = ref_audio_out_tx_channels.borrow_mut().to_vec();
	let d = Dispatcher::new(
	    ps_rx,
	    audio_out_tx_channels
	);
	d.start().await;
    }
}
