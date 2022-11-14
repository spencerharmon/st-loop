use jack::jack_sys as j;
use tokio::task;
use crossbeam_channel::*;
use std::mem::MaybeUninit;
use std::{thread, time};
use crate::looper::Looper;
use st_lib::owned_midi::*;
use crate::scene::Scene;
use std::rc::Rc;
use std::cell::RefCell;

pub struct JackIO;

impl JackIO {
    pub fn new() -> JackIO {
        JackIO { }
    }
    pub async fn start(self)  {
	//signals once per process cycle
        let (ps_tx, ps_rx) = bounded(1);

	//used by CommandManager
	let (command_midi_tx, command_midi_rx) = bounded(100);

	//dummy vec of midi senders
	let midi_tx_channels = Vec::new();
	//dummy vec of midi receivers
	let midi_rx_channels = Vec::new();
	
	//audio channels
	let audio_channel_count = 8;
	let audio_tx_channels = Rc::new(RefCell::new(Vec::new()));
	let audio_rx_channels = Rc::new(RefCell::new(Vec::new()));
	for i in 0..audio_channel_count {
	    let (tx, rx) = unbounded();
	    
	    audio_rx_channels.borrow_mut().push(rx);
	    audio_tx_channels.borrow_mut().push(tx);
	}
        let (client, _status) =
            jack::Client::new("st-loop", jack::ClientOptions::NO_START_SERVER).unwrap();
        let mut command_midi_port = client
            .register_port("command", jack::MidiIn::default())
            .unwrap();
	let client_pointer = client.raw();

	let process = jack::ClosureProcessHandler::new(
            move |client: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
                match ps_tx.try_send(()) {
		    Ok(()) => (),
		    Err(_) => ()
		}

		// Get output buffer
		let mut command_midi_in = command_midi_port.iter(ps);

		for s in command_midi_in{
		    let om = OwnedMidi { time: s.time, bytes: s.bytes.to_owned() };
		    command_midi_tx.try_send(om);
		}
		

                jack::Control::Continue
            },
        );
        let active_client = client.activate_async((), process).unwrap();

	let mut looper = Looper::new(
	    command_midi_rx,
	    audio_rx_channels,
	    midi_rx_channels,
	    audio_tx_channels,
	    midi_tx_channels,
	    client_pointer.expose_addr()
	);
	looper.start().await;
    }
}
