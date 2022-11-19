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
use std::cell::RefMut;

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
	
        let (client, _status) =
            jack::Client::new("st-loop", jack::ClientOptions::NO_START_SERVER).unwrap();
	
	//audio channels
	let audio_channel_count = 8;
	// use ref cell to create in loop
	let mut audio_out_tx_channels = Vec::<(Sender<f32>, Sender<f32>)>::new();
	let mut audio_out_rx_channels = Vec::new();
	let mut audio_in_tx_channels = Vec::new();
	let mut audio_in_rx_channels = Vec::new();
	let mut audio_in_jack_ports = Vec::new();
//	let mut audio_out_jack_ports = Vec::new();

	let mut in_0_l = client
	    .register_port("in_0_l", jack::AudioIn::default())
	    .unwrap();
	let mut in_0_r = client
	    .register_port("in_0_r", jack::AudioIn::default())
	    .unwrap();
	let mut in_1_l = client
	    .register_port("in_1_l", jack::AudioIn::default())
	    .unwrap();
	let mut in_1_r = client
	    .register_port("in_1_r", jack::AudioIn::default())
	    .unwrap();

	audio_in_jack_ports.push((in_0_l, in_0_r));
	audio_in_jack_ports.push((in_1_l, in_1_r));

	let mut out_0_l = client
	    .register_port("out_0_l", jack::AudioOut::default())
	    .unwrap();
	let mut out_0_r = client
	    .register_port("out_0_r", jack::AudioOut::default())
	    .unwrap();
	let mut out_1_l = client
	    .register_port("out_1_l", jack::AudioOut::default())
	    .unwrap();
	let mut out_1_r = client
	    .register_port("out_1_r", jack::AudioOut::default())
	    .unwrap();

//	audio_out_jack_ports.push((out_0_l, out_0_r));
//	audio_out_jack_ports.push((out_1_l, out_1_r));

	//channel 0
	let (out_l_tx_0, out_l_rx_0) = bounded(1000);
	let (out_r_tx_0, out_r_rx_0) = bounded(1000);
	let (in_tx_0, in_rx_0) = bounded(1000);
	
//	audio_out_rx_channels.push((out_l_rx_0, out_r_rx_0));
	audio_out_tx_channels.push((out_l_tx_0, out_r_tx_0));
	audio_in_rx_channels.push(in_rx_0);
	audio_in_tx_channels.push(in_tx_0);
	
	//channel 1
	let (out_l_tx_1, out_l_rx_1) = bounded(1000);
	let (out_r_tx_1, out_r_rx_1) = bounded(1000);
	let (in_tx_1, in_rx_1) = bounded(1000);
	
	audio_out_rx_channels.push((out_l_rx_1, out_r_rx_1));
	audio_out_tx_channels.push((out_l_tx_1, out_r_tx_1));
	audio_in_rx_channels.push(in_rx_1);
	audio_in_tx_channels.push(in_tx_1);

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


		    // jack input; split tuple
		    let (jack_l, jack_r) = audio_in_jack_ports.get(0).unwrap();

		    let mut in_l = jack_l.as_slice(ps);
		    let mut in_r = jack_r.as_slice(ps);

		    for i in 0..in_l.len() {
			// receive input from jack, send to looper via channel
			match in_l.get(i) {
			    Some(l_bytes) => {
				match in_r.get(i) {
				    Some(r_bytes) => {
					audio_in_tx_channels.get(0)
					    .unwrap()
					    .try_send(
						(*l_bytes, *r_bytes)
					    );
				    },
				    _ => ()
				}
			    },
			    _ => ()
			}
		    }
		    
		    


		//channel by bloody channel

		// loop output; split tuple

//		let (l_out_chan0, r_out_chan0) = audio_out_rx_channels.get(0).unwrap();

		// write left output
		for v in out_0_l.as_mut_slice(ps).iter_mut(){
		    *v = 0.0;
		    if let Ok(float) = out_l_rx_0.try_recv() {
//			println!("{}", float);
                        *v = float;
		    }
		}
		// write right output
		for v in out_0_r.as_mut_slice(ps).iter_mut(){
		    *v = 0.0;
		    if let Ok(float) = out_r_rx_0.try_recv() {
//			println!("{}", float);
                        *v = float;
		    }
		}

                let mut jack_out_l = out_1_l.as_mut_slice(ps);
                let mut jack_out_r = out_1_r.as_mut_slice(ps);
		//...blablabla
		
                jack::Control::Continue
            },
        );
        let active_client = client.activate_async((), process).unwrap();

	let mut looper = Looper::new(
	    ps_rx,
	    command_midi_rx,
	    audio_in_rx_channels,
	    midi_rx_channels,
	    audio_out_tx_channels,
	    midi_tx_channels,
	    client_pointer.expose_addr()
	);
	looper.start().await;
    }
}
