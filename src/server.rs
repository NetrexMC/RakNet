use std::{ thread, net::{ SocketAddr }, sync::{ Arc, mpsc::* } };
// use tokio::net::UdpSocket;
use std::net::UdpSocket;
// use rand::random;
use binary_utils::*;
use super::conn::Connection;
use crate::{ util::* };

pub const THREAD_QUIT: i64 = 982939013934;

pub trait IRakServer {
     fn new(address: String, version: u8) -> Self;
     fn start(&mut self) -> (&mut RakServer, Arc<RakEmitter>);
}

pub struct RakServer {
     address: String,
     version: u8,
     connections: Vec<Connection>
}

impl IRakServer for RakServer {
     fn new(address: String, version: u8) -> Self {
          Self {
               version,
               address,
               connections: Vec::new()
          }
     }

     fn start(&mut self) -> (&mut RakServer, Arc<RakEmitter>) {
          return start(self);
     }
}

pub fn start(serv: &mut RakServer) -> (&mut RakServer, Arc<RakEmitter>) {
     let socket = UdpSocket::bind(serv.address.parse::<SocketAddr>().unwrap());
     let resource: Arc<UdpSocket> = Arc::new(socket.unwrap());
     let res = Arc::clone(&resource);
     let ev_channel = RakEmitter::new();
     let ec1 = Arc::new(ev_channel);
     let ec2 = Arc::clone(&ec1);

     // ServerBound
     thread::spawn(move || {
          let mut buf = [0; 65535];
          loop {
               let (len, rem) = match resource.as_ref().recv_from(&mut buf) {
                    Ok(v) => v,
                    Err(_e) => {
                         continue;
                    }
               };

               let data = &buf[..len];
               let ev = RakEv::Recieve(rem, BinaryStream::init(&data.to_vec()));
               ec1.as_ref().broadcast(&ev);
          }
     });

     // mspc channels recievers don't work on threads?
     let (_sr, rc) = channel::<(SocketAddr, BinaryStream)>();

     thread::spawn(move || {
          loop {
               let (address, stream) = match rc.try_recv() {
                    Ok(t) => t,
                    Err(_e) => {
                         println!("Could not recieve any data from mspc as channel is probably closed.");
                         continue;
                    }
               };
               let mut st = BinaryStream::init(&stream.get_buffer());

               if address.ip().is_loopback() && st.read_byte() == 0xCE && st.read_long() == THREAD_QUIT {
                    println!("Recieved quit message");
                    break;
               }

               res.as_ref().send_to(&*stream.get_buffer(), address).expect("Could not send bytes to client.");
          }
     });

     return (serv, ec2);
}