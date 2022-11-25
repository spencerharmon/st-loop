use jack::jack_sys as j;
use tokio::task;
use crossbeam_channel::*;
use std::mem::MaybeUninit;
use std::{thread, time};
use crate::looper::Looper;
use st_lib::owned_midi::*;
use crate::scene::Scene;
use crate::constants::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::cell::RefMut;

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

	//jack input ports
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
	let mut in_2_l = client
	    .register_port("in_2_l", jack::AudioIn::default())
	    .unwrap();
	let mut in_2_r = client
	    .register_port("in_2_r", jack::AudioIn::default())
	    .unwrap();
	let mut in_3_l = client
	    .register_port("in_3_l", jack::AudioIn::default())
	    .unwrap();
	let mut in_3_r = client
	    .register_port("in_3_r", jack::AudioIn::default())
	    .unwrap();
	let mut in_4_l = client
	    .register_port("in_4_l", jack::AudioIn::default())
	    .unwrap();
	let mut in_4_r = client
	    .register_port("in_4_r", jack::AudioIn::default())
	    .unwrap();
	let mut in_5_l = client
	    .register_port("in_5_l", jack::AudioIn::default())
	    .unwrap();
	let mut in_5_r = client
	    .register_port("in_5_r", jack::AudioIn::default())
	    .unwrap();
	let mut in_6_l = client
	    .register_port("in_6_l", jack::AudioIn::default())
	    .unwrap();
	let mut in_6_r = client
	    .register_port("in_6_r", jack::AudioIn::default())
	    .unwrap();
	let mut in_7_l = client
	    .register_port("in_7_l", jack::AudioIn::default())
	    .unwrap();
	let mut in_7_r = client
	    .register_port("in_7_r", jack::AudioIn::default())
	    .unwrap();

	audio_in_jack_ports.push((in_0_l, in_0_r));
	audio_in_jack_ports.push((in_1_l, in_1_r));
	audio_in_jack_ports.push((in_2_l, in_2_r));
	audio_in_jack_ports.push((in_3_l, in_3_r));
	audio_in_jack_ports.push((in_4_l, in_4_r));
	audio_in_jack_ports.push((in_5_l, in_5_r));
	audio_in_jack_ports.push((in_6_l, in_6_r));
	audio_in_jack_ports.push((in_7_l, in_7_r));

	//jack output ports
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
	let mut out_2_l = client
	    .register_port("out_2_l", jack::AudioOut::default())
	    .unwrap();
	let mut out_2_r = client
	    .register_port("out_2_r", jack::AudioOut::default())
	    .unwrap();
	let mut out_3_l = client
	    .register_port("out_3_l", jack::AudioOut::default())
	    .unwrap();
	let mut out_3_r = client
	    .register_port("out_3_r", jack::AudioOut::default())
	    .unwrap();
	let mut out_4_l = client
	    .register_port("out_4_l", jack::AudioOut::default())
	    .unwrap();
	let mut out_4_r = client
	    .register_port("out_4_r", jack::AudioOut::default())
	    .unwrap();
	let mut out_5_l = client
	    .register_port("out_5_l", jack::AudioOut::default())
	    .unwrap();
	let mut out_5_r = client
	    .register_port("out_5_r", jack::AudioOut::default())
	    .unwrap();
	let mut out_6_l = client
	    .register_port("out_6_l", jack::AudioOut::default())
	    .unwrap();
	let mut out_6_r = client
	    .register_port("out_6_r", jack::AudioOut::default())
	    .unwrap();
	let mut out_7_l = client
	    .register_port("out_7_l", jack::AudioOut::default())
	    .unwrap();
	let mut out_7_r = client
	    .register_port("out_7_r", jack::AudioOut::default())
	    .unwrap();


	//channel 0
	let (out_l_tx_0, out_l_rx_0) = bounded(1000);
	let (out_r_tx_0, out_r_rx_0) = bounded(1000);
	let (in_tx_0, in_rx_0) = bounded(1000);
	
	audio_out_rx_channels.push((&out_l_rx_0, &out_r_rx_0));
	audio_out_tx_channels.push((out_l_tx_0, out_r_tx_0));
	audio_in_rx_channels.push(in_rx_0);
	audio_in_tx_channels.push(in_tx_0);
	
	//channel 1
	let (out_l_tx_1, out_l_rx_1) = bounded(1000);
	let (out_r_tx_1, out_r_rx_1) = bounded(1000);
	let (in_tx_1, in_rx_1) = bounded(1000);
	
	audio_out_rx_channels.push((&out_l_rx_1, &out_r_rx_1));
	audio_out_tx_channels.push((out_l_tx_1, out_r_tx_1));
	audio_in_rx_channels.push(in_rx_1);
	audio_in_tx_channels.push(in_tx_1);

	//channel 2
	let (out_l_tx_2, out_l_rx_2) = bounded(1000);
	let (out_r_tx_2, out_r_rx_2) = bounded(1000);
	let (in_tx_2, in_rx_2) = bounded(1000);
	
	audio_out_rx_channels.push((&out_l_rx_2, &out_r_rx_2));
	audio_out_tx_channels.push((out_l_tx_2, out_r_tx_2));
	audio_in_rx_channels.push(in_rx_2);
	audio_in_tx_channels.push(in_tx_2);

	//channel 3
	let (out_l_tx_3, out_l_rx_3) = bounded(1000);
	let (out_r_tx_3, out_r_rx_3) = bounded(1000);
	let (in_tx_3, in_rx_3) = bounded(1000);
	
	audio_out_rx_channels.push((&out_l_rx_3, &out_r_rx_3));
	audio_out_tx_channels.push((out_l_tx_3, out_r_tx_3));
	audio_in_rx_channels.push(in_rx_3);
	audio_in_tx_channels.push(in_tx_3);

	//channel 4
	let (out_l_tx_4, out_l_rx_4) = bounded(1000);
	let (out_r_tx_4, out_r_rx_4) = bounded(1000);
	let (in_tx_4, in_rx_4) = bounded(1000);
	
	audio_out_rx_channels.push((&out_l_rx_4, &out_r_rx_4));
	audio_out_tx_channels.push((out_l_tx_4, out_r_tx_4));
	audio_in_rx_channels.push(in_rx_4);
	audio_in_tx_channels.push(in_tx_4);

	//channel 5
	let (out_l_tx_5, out_l_rx_5) = bounded(1000);
	let (out_r_tx_5, out_r_rx_5) = bounded(1000);
	let (in_tx_5, in_rx_5) = bounded(1000);
	
	audio_out_rx_channels.push((&out_l_rx_5, &out_r_rx_5));
	audio_out_tx_channels.push((out_l_tx_5, out_r_tx_5));
	audio_in_rx_channels.push(in_rx_5);
	audio_in_tx_channels.push(in_tx_5);

	//channel 6
	let (out_l_tx_6, out_l_rx_6) = bounded(1000);
	let (out_r_tx_6, out_r_rx_6) = bounded(1000);
	let (in_tx_6, in_rx_6) = bounded(1000);
	
	audio_out_rx_channels.push((&out_l_rx_6, &out_r_rx_6));
	audio_out_tx_channels.push((out_l_tx_6, out_r_tx_6));
	audio_in_rx_channels.push(in_rx_6);
	audio_in_tx_channels.push(in_tx_6);

	//channel 7
	let (out_l_tx_7, out_l_rx_7) = bounded(1000);
	let (out_r_tx_7, out_r_rx_7) = bounded(1000);
	let (in_tx_7, in_rx_7) = bounded(1000);
	
	audio_out_rx_channels.push((&out_l_rx_7, &out_r_rx_7));
	audio_out_tx_channels.push((out_l_tx_7, out_r_tx_7));
	audio_in_rx_channels.push(in_rx_7);
	audio_in_tx_channels.push(in_tx_7);
	
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
		    if let Some(b) = recording.get(t) {
			if *b == false {
			    continue
			}
		    }
		    // jack input; split tuple
                    let (jack_l, jack_r) = audio_in_jack_ports.get(t).unwrap();
		    
                    let mut in_l = jack_l.as_slice(ps);
                    let mut in_r = jack_r.as_slice(ps);
		    
                    for i in 0..in_l.len() {
                        // receive input from jack, send to looper via channel
                        if let Some(l_bytes) = in_l.get(i) {
                            if let Some(r_bytes) = in_r.get(i) {
                                audio_in_tx_channels.get(t)
                                    .unwrap()
                                    .try_send(
                                        (*l_bytes, *r_bytes)
                                    );
                            }
                        }
                    }
		}    


		//channel by bloody channel

		//track 0
		if let Some(b) = playing.get(0) {
		    if *b {

			let mut end = false;

			// write left output
			for v in out_0_l.as_mut_slice(ps).iter_mut(){
			    if end {
				*v = 0.0;
			    } else {
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
			for v in out_0_r.as_mut_slice(ps).iter_mut(){
			    if end {
				*v = 0.0;
			    } else {
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

		//track 1
		if let Some(b) = playing.get(1) {
		    if *b {

			let mut end = false;

			// write left output
			for v in out_1_l.as_mut_slice(ps).iter_mut(){
			    if end {
				*v = 0.0;
			    } else {
				match out_l_rx_1.try_recv() {
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
			for v in out_1_r.as_mut_slice(ps).iter_mut(){
			    if end {
				*v = 0.0;
			    } else {
				match out_r_rx_1.try_recv() {
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
		
		//track 2
		if let Some(b) = playing.get(2) {
		    if *b {

			let mut end = false;

			// write left output
			for v in out_2_l.as_mut_slice(ps).iter_mut(){
			    if end {
				*v = 0.0;
			    } else {
				match out_l_rx_2.try_recv() {
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
			for v in out_2_r.as_mut_slice(ps).iter_mut(){
			    if end {
				*v = 0.0;
			    } else {
				match out_r_rx_2.try_recv() {
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
		//track 3
		if let Some(b) = playing.get(3) {
		    if *b {

			let mut end = false;

			// write left output
			for v in out_3_l.as_mut_slice(ps).iter_mut(){
			    if end {
				*v = 0.0;
			    } else {
				match out_l_rx_3.try_recv() {
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
			for v in out_3_r.as_mut_slice(ps).iter_mut(){
			    if end {
				*v = 0.0;
			    } else {
				match out_r_rx_3.try_recv() {
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
		//track 4
		if let Some(b) = playing.get(4) {
		    if *b {

			let mut end = false;

			// write left output
			for v in out_4_l.as_mut_slice(ps).iter_mut(){
			    if end {
				*v = 0.0;
			    } else {
				match out_l_rx_4.try_recv() {
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
			for v in out_4_r.as_mut_slice(ps).iter_mut(){
			    if end {
				*v = 0.0;
			    } else {
				match out_r_rx_4.try_recv() {
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
		//track 5
		if let Some(b) = playing.get(5) {
		    if *b {

			let mut end = false;

			// write left output
			for v in out_5_l.as_mut_slice(ps).iter_mut(){
			    if end {
				*v = 0.0;
			    } else {
				match out_l_rx_5.try_recv() {
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
			for v in out_5_r.as_mut_slice(ps).iter_mut(){
			    if end {
				*v = 0.0;
			    } else {
				match out_r_rx_5.try_recv() {
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
		
		//track 6
		if let Some(b) = playing.get(6) {
		    if *b {

			let mut end = false;

			// write left output
			for v in out_6_l.as_mut_slice(ps).iter_mut(){
			    if end {
				*v = 0.0;
			    } else {
				match out_l_rx_6.try_recv() {
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
			for v in out_6_r.as_mut_slice(ps).iter_mut(){
			    if end {
				*v = 0.0;
			    } else {
				match out_r_rx_6.try_recv() {
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
		//track 7
		if let Some(b) = playing.get(7) {
		    if *b {

			let mut end = false;

			// write left output
			for v in out_7_l.as_mut_slice(ps).iter_mut(){
			    if end {
				*v = 0.0;
			    } else {
				match out_l_rx_7.try_recv() {
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
			for v in out_7_r.as_mut_slice(ps).iter_mut(){
			    if end {
				*v = 0.0;
			    } else {
				match out_r_rx_7.try_recv() {
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
                jack::Control::Continue
            },
        );
        let active_client = client.activate_async((), process).unwrap();

	let mut looper = Looper::new(
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
	looper.start().await;
    }
}
