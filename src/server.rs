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

     fn start(&mut self);
}

pub struct RakServer {
     address: String,
     version: u8,
     connections: Vec<Connection>,
     channel: RakEventListener,
}

impl IRakServer for RakServer {
     fn new(address: String, version: u8) -> Self {
          Self {
               version,
               address,
               connections: Vec::new(),
               channel: RakEventListener::new()
          }
     }

     fn start(&mut self) {
          let socket = UdpSocket::bind(self.address.parse::<SocketAddr>().unwrap());
          let resource: Arc<UdpSocket> = Arc::new(socket.unwrap());
          let res = Arc::clone(&resource);
          let channels = (Arc::new(&self.channel), Arc::new(&self.channel)); // we'll use this across threads.

          // ServerBound
          thread::spawn(move || {
               let mut buf = [0; 65535];
               loop {
                    let (len, rem) = match resource.as_ref().recv_from(&mut buf) {
                         Ok(v) => v,
                         Err(e) => {
                              continue;
                         }
                    };

                    let data = &buf[..len];
                    channels.0.as_ref().broadcast(RakEvent::Recieve(rem, BinaryStream::init(&data.to_vec())));
               }
          });

          // thing to get shit from server
          let (sr, rc) = channel::<(SocketAddr, BinaryStream)>();

          thread::spawn(move || {
               loop {
                    let (address, stream) = match rc.try_recv() {
                         Ok(t) => t,
                         Err(TryRecvError) => {
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
     }
}