use jack::jack_sys as j;
use tokio::task;
use crossbeam_channel::*;
use std::mem::MaybeUninit;
use std::{thread, time};
use crate::dispatcher::Dispatcher;
use st_lib::owned_midi::*;
use crate::scene::Scene;
use crate::constants::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::cell::RefMut;
use tokio::sync::mpsc;

pub struct JackIO;

impl JackIO {
    pub fn new() -> JackIO {
        JackIO { }
    }
    pub async fn start(self)  {

	//signals jack callback to start and stop sending data from a track
        let (start_recording_tx, start_recording_rx) = bounded(100);
        let (stop_recording_tx, stop_recording_rx) = bounded(100);

	//same, but for playing tracks
        let (start_playing_tx, start_playing_rx) = bounded(100);
        let (stop_playing_tx, stop_playing_rx) = bounded(100);

	//used by CommandManager
	let (command_midi_tx, command_midi_rx) = bounded(100);

	//dummy vec of midi senders
	let midi_tx_channels = Vec::new();
	//dummy vec of midi receivers
	let midi_rx_channels = Vec::new();
	
        let (client, _status) =
            jack::Client::new(CLIENT_NAME, jack::ClientOptions::NO_START_SERVER).unwrap();
	
	//audio channels
	let audio_channel_count = 8;
	// use ref cell to create in loop
	let mut audio_out_tx_channels = Vec::<(Sender<f32>, Sender<f32>)>::new();
	let mut audio_out_rx_channels = Vec::new();
	let mut audio_in_tx_channels = Vec::new();
	let mut audio_in_rx_channels = Vec::new();
	let mut audio_in_jack_ports = Vec::new();
	let mut audio_out_jack_ports = Vec::<(
	    jack::Port<jack::AudioOut>,
	    jack::Port<jack::AudioOut>)>::new();


	let ref_audio_in_ports = RefCell::new(audio_in_jack_ports);
	let ref_audio_out_ports = RefCell::new(audio_out_jack_ports);
	for i in 0..AUDIO_TRACK_COUNT {
	    let mut b_audio_in_ports = ref_audio_in_ports.borrow_mut();
	    let mut b_audio_out_ports = ref_audio_out_ports.borrow_mut();
	    let mut in_l = client
		.register_port(format!("in_{i}_l").as_str(), jack::AudioIn::default())
		.unwrap();
	    let mut in_r = client
		.register_port(format!("in_{i}_r").as_str(), jack::AudioIn::default())
		.unwrap();
	    b_audio_in_ports.push((in_l, in_r));
	
	    //jack output ports
	    let mut out_l = client
		.register_port(format!("out_{i}_l").as_str(), jack::AudioOut::default())
	    .unwrap();
	    let mut out_r = client
		.register_port(format!("out_{i}_r").as_str(), jack::AudioOut::default())
		.unwrap();
	    b_audio_out_ports.push((out_l, out_r));
	}

	//channel 0
	let (out_l_tx_0, out_l_rx_0) = unbounded();
	let (out_r_tx_0, out_r_rx_0) = unbounded();
	let (in_tx_0, in_rx_0) = unbounded();
	
	audio_out_rx_channels.push((&out_l_rx_0, &out_r_rx_0));
	audio_out_tx_channels.push((out_l_tx_0, out_r_tx_0));
	audio_in_rx_channels.push(in_rx_0);
	audio_in_tx_channels.push(in_tx_0);
	
	//channel 1
	let (out_l_tx_1, out_l_rx_1) = unbounded();
	let (out_r_tx_1, out_r_rx_1) = unbounded();
	let (in_tx_1, in_rx_1) = unbounded();
	
	audio_out_rx_channels.push((&out_l_rx_1, &out_r_rx_1));
	audio_out_tx_channels.push((out_l_tx_1, out_r_tx_1));
	audio_in_rx_channels.push(in_rx_1);
	audio_in_tx_channels.push(in_tx_1);

	//channel 2
	let (out_l_tx_2, out_l_rx_2) = unbounded();
	let (out_r_tx_2, out_r_rx_2) = unbounded();
	let (in_tx_2, in_rx_2) = unbounded();
	
	audio_out_rx_channels.push((&out_l_rx_2, &out_r_rx_2));
	audio_out_tx_channels.push((out_l_tx_2, out_r_tx_2));
	audio_in_rx_channels.push(in_rx_2);
	audio_in_tx_channels.push(in_tx_2);

	//channel 3
	let (out_l_tx_3, out_l_rx_3) = unbounded();
	let (out_r_tx_3, out_r_rx_3) = unbounded();
	let (in_tx_3, in_rx_3) = unbounded();
	
	audio_out_rx_channels.push((&out_l_rx_3, &out_r_rx_3));
	audio_out_tx_channels.push((out_l_tx_3, out_r_tx_3));
	audio_in_rx_channels.push(in_rx_3);
	audio_in_tx_channels.push(in_tx_3);

	//channel 4
	let (out_l_tx_4, out_l_rx_4) = unbounded();
	let (out_r_tx_4, out_r_rx_4) = unbounded();
	let (in_tx_4, in_rx_4) = unbounded();
	
	audio_out_rx_channels.push((&out_l_rx_4, &out_r_rx_4));
	audio_out_tx_channels.push((out_l_tx_4, out_r_tx_4));
	audio_in_rx_channels.push(in_rx_4);
	audio_in_tx_channels.push(in_tx_4);

	//channel 5
	let (out_l_tx_5, out_l_rx_5) = unbounded();
	let (out_r_tx_5, out_r_rx_5) = unbounded();
	let (in_tx_5, in_rx_5) = unbounded();
	
	audio_out_rx_channels.push((&out_l_rx_5, &out_r_rx_5));
	audio_out_tx_channels.push((out_l_tx_5, out_r_tx_5));
	audio_in_rx_channels.push(in_rx_5);
	audio_in_tx_channels.push(in_tx_5);

	//channel 6
	let (out_l_tx_6, out_l_rx_6) = unbounded();
	let (out_r_tx_6, out_r_rx_6) = unbounded();
	let (in_tx_6, in_rx_6) = unbounded();
	
	audio_out_rx_channels.push((&out_l_rx_6, &out_r_rx_6));
	audio_out_tx_channels.push((out_l_tx_6, out_r_tx_6));
	audio_in_rx_channels.push(in_rx_6);
	audio_in_tx_channels.push(in_tx_6);

	//channel 7
	let (out_l_tx_7, out_l_rx_7) = unbounded();
	let (out_r_tx_7, out_r_rx_7) = unbounded();
	let (in_tx_7, in_rx_7) = unbounded();
	
	audio_out_rx_channels.push((&out_l_rx_7, &out_r_rx_7));
	audio_out_tx_channels.push((out_l_tx_7, out_r_tx_7));
	audio_in_rx_channels.push(in_rx_7);
	audio_in_tx_channels.push(in_tx_7);

	let (ps_tx, ps_rx) = mpsc::channel(1);
        let mut command_midi_port = client
            .register_port("command", jack::MidiIn::default())
            .unwrap();
	let client_pointer = client.raw();

	let mut recording: Vec<bool> = Vec::new();
	for _ in 0..AUDIO_TRACK_COUNT {
	    recording.push(false);
	}
	let mut playing: Vec<bool> = Vec::new();
	for _ in 0..AUDIO_TRACK_COUNT {
	    playing.push(false);
	}
	let process = jack::ClosureProcessHandler::new(
            move |client: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {

		let mut b_audio_in_ports = ref_audio_in_ports.borrow_mut();
		let mut b_audio_out_ports = ref_audio_out_ports.borrow_mut();
		
                match ps_tx.try_send(()) {
		    Ok(()) => (),
		    Err(_) => ()
		}
		ps_tx.try_send(());
		//set recording tracks
		loop {
		    if let Ok(track) = start_recording_rx.try_recv(){
			if let Some(b) = recording.get_mut(track) {
			    *b = true;
			}
		    } else {
			break
		    }
		}

		loop {
		    if let Ok(track) = stop_recording_rx.try_recv(){
			if let Some(b) = recording.get_mut(track) {
			    *b = false;
			}
		    } else {
			break
		    }
		}		
		//set playing tracks
		loop {
		    if let Ok(track) = start_playing_rx.try_recv(){
			println!("Start playing: {}", track);
			if let Some(b) = playing.get_mut(track) {
			    *b = true;
			}
		    } else {
			break
		    }
		}

		loop {
		    if let Ok(track) = stop_playing_rx.try_recv(){
			println!("Stop playing: {}", track);
			if let Some(b) = playing.get_mut(track) {
			    *b = false;
			}
		    } else {
			break
		    }
		}		

		let mut command_midi_in = command_midi_port.iter(ps);
		for s in command_midi_in{
		    let om = OwnedMidi { time: s.time, bytes: s.bytes.to_owned() };
		    command_midi_tx.try_send(om);
		}

		for t in 0..AUDIO_TRACK_COUNT {
		    //record/in
		    if let Some(b) = recording.get(t) {
			if *b == false {
			    continue
			}
		    }
		    // jack input; split tuple
                    let (jack_l, jack_r) = b_audio_in_ports.get(t).unwrap();
		    
                    let mut in_l = jack_l.as_slice(ps);
                    let mut in_r = jack_r.as_slice(ps);
		    
                    for i in 0..in_l.len() {
                        // receive input from jack, send to looper via channel
                        if let Some(l_bytes) = in_l.get(i) {
                            if let Some(r_bytes) = in_r.get(i) {
                                audio_in_tx_channels.get(t)
                                    .unwrap()
                                    .send(
                                        (*l_bytes, *r_bytes)
                                    );
                            }
                        }
                    }

		    //play/out
		    if let Some(b) = playing.get(t) {
			if *b {
			    let (ref mut out_l, ref mut out_r) = b_audio_out_ports.get_mut(t).unwrap();

			    let mut end = false;

			    // write left output
			    for v in out_l.as_mut_slice(ps).iter_mut(){
				if end {
				    *v = 0.0;
				} else {
				    //todo this is wrong. needs to get from a vec of rx channels
				    match out_l_rx_0.try_recv() {
					Ok(float) => *v = float,
					Err(_) => {
					    *v = 0.0;
					    end = true;
					}
				    }
				}
			    }

			    // write right output
			    end = false;
			    for v in out_r.as_mut_slice(ps).iter_mut(){
				if end {
				    *v = 0.0;
				} else {
				    //todo this is wrong. needs to get from a vec of rx channels
				    match out_r_rx_0.try_recv() {
					Ok(float) => *v = float,
					Err(_) => {
					    *v = 0.0;
					    end = true;
					}
				    }
				}
			    }
			}
		    }
    		    
		}
                jack::Control::Continue
            },
        );
        let active_client = client.activate_async((), process).unwrap();

	let mut dispatcher = Dispatcher::new(
	    ps_rx,
	    start_playing_tx,
	    stop_playing_tx,
	    start_recording_tx,
	    stop_recording_tx,
	    command_midi_rx,
	    audio_in_rx_channels,
	    midi_rx_channels,
	    audio_out_tx_channels,
	    midi_tx_channels,
	    client_pointer.expose_addr()
	);
	dispatcher.start().await;
    }
}
