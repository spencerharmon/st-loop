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
    tx: Sender<()>
}
impl ClientConnection {
    fn new(tx: Sender<()>) -> ClientConnection {
	
	ClientConnection { tx }
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
		    println!("save");
		    self.tx.try_send(());
		    sock.send_to(&get_reply(msg), to_addr).unwrap();
		}
		"/nsm/client/open" => {
		    println!("open");
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
    rx: Receiver<()>
}

impl Client {
    pub fn new() -> Client {
        let (tx, rx) = bounded(1);
	let cc = ClientConnection::new(tx);
	tokio::task::spawn(async move {
	    cc.start().await 
	});
	
	Client { rx }
    }

    fn try_recv_save(self) -> Result<(), TryRecvError> {
	self.rx.try_recv()
    }
    
}
