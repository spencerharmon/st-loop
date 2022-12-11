use crate::constants::*;
use std::{env, process};
use crossbeam_channel::*;

use rosc::{OscMessage, OscPacket, OscType};
use rosc::encoder;
use std::net::{SocketAddrV4, UdpSocket};

fn get_reply(msg: OscMessage) -> Vec<u8> {
    encoder::encode(&OscPacket::Message(OscMessage {
		    addr: "/reply".to_string(),
		    args: vec![
			OscType::from(msg.addr),
			OscType::from("hello from st-loop"),
		    ],
		})).unwrap()
}
struct ClientConnection {
    save_tx: Sender<String>,
    open_tx: Sender<String>
}
impl ClientConnection {
    fn new(save_tx: Sender<String>, open_tx: Sender<String>) -> ClientConnection {
	ClientConnection { save_tx, open_tx }
    }
    async fn start (self) {
	if let Ok(nsm_url) = env::var("NSM_URL") {
	    println!("Connecting to {}", nsm_url);
	    let addr_parts: Vec<&str> = nsm_url.split("/").collect();
	    let to_addr = addr_parts[2];

	    let sock = UdpSocket::bind("127.0.0.1:0").unwrap();

	    let msg_buf = encoder::encode(&OscPacket::Message(OscMessage {
                addr: "/nsm/server/announce".to_string(),
                args: vec![
		    OscType::from(CLIENT_NAME),
		    OscType::from(""),
		    OscType::from(CLIENT_NAME),		    
		    OscType::from(1),
		    OscType::from(1),
		    OscType::from(process::id() as i32),
			
		],
            }))
            .unwrap();
        
            sock.send_to(&msg_buf, to_addr).unwrap();


	    let mut buf = [0u8; rosc::decoder::MTU];
	    loop {
                match sock.recv_from(&mut buf) {
                    Ok((size, addr)) => {
                        let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
                        self.process_packet(packet, &sock, to_addr);
                    }
                    Err(e) => {
                        println!("Error receiving from socket: {}", e);
                        break;
                    }
                }
	    }
	}
    }
    fn process_packet(&self, packet: OscPacket, sock: &UdpSocket, to_addr: &str) {
	if let OscPacket::Message(msg) = packet {
	    match msg.addr.as_str() {
		"/nsm/client/save" => {
		    println!("save {:?}", msg.args);
		    self.save_tx.try_send("booo".to_string());
		    sock.send_to(&get_reply(msg), to_addr).unwrap();
		}
		"/nsm/client/open" => {
		    println!("open {:?}", msg.args);
		    let s = msg.args[0].clone().string().unwrap();
		    self.open_tx.try_send(s);
		    sock.send_to(&get_reply(msg), to_addr).unwrap();
		}
		_ => {
//		    println!("NO MATCH");
//		    println!("{:?}", msg.addr);
//		    println!("{:?}", msg.args);
		}
	    }
	}
    }
}

pub struct Client {
    save_rx: Receiver<String>,
    open_rx: Receiver<String>
}

impl Client {
    pub fn new() -> Client {
        let (save_tx, save_rx) = bounded(1);
        let (open_tx, open_rx) = bounded(1);
	let cc = ClientConnection::new(save_tx, open_tx);
	tokio::task::spawn(async move {
	    cc.start().await 
	});
	
	Client { save_rx, open_rx }
    }

    pub fn try_recv_save(&self) -> Result<String, TryRecvError> {
	self.save_rx.try_recv()
    }
    pub fn try_recv_open(&self) -> Result<String, TryRecvError> {
	self.open_rx.try_recv()
    }
}
