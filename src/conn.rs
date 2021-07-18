use std::net::SocketAddr;
use std::collections::VecDeque;
use std::time::SystemTime;
use crate::{ Motd };
use crate::protocol::offline::*;
use binary_utils::*;

pub type RecievePacketFn = dyn FnMut(&mut Connection, &mut BinaryStream) -> std::io::Result<()>;

pub trait ConnectionAPI {
     /// Called when a packet is recieved from raknet
     /// This is called on each **Frame**
     fn recieve_packet(&mut self, stream: &mut BinaryStream);

     /// Called when RakNet wants to generate a **Motd**
     /// for the server, if this fails, the `default_motd`
     /// function is called instead.
     fn gen_motd(&mut self) -> Motd;
}

#[derive(Clone)]
pub struct Connection {
     // read by raknet
     pub send_queue: VecDeque<BinaryStream>,
     pub connected: bool,
     pub address: SocketAddr,
     pub time: SystemTime,
     pub motd: Motd
}

impl Connection {
     pub fn new(address: SocketAddr, start_time: SystemTime) -> Self {
          Self {
               send_queue: VecDeque::new(),
               connected: false,
               address,
               time: start_time,
               motd: Motd::default()
          }
     }

     /// Used internally by raknet for **each** packet recieved
     pub fn receive(&mut self, stream: &mut BinaryStream) {
          // They are not connected, perform connection sequence
          if !self.connected {
               let pk = OfflinePackets::recv(stream.read_byte());
               let handler = handle_offline(self, pk, stream);

               self.send_queue.push_back(handler.clone());
          } else {
          }
     }
}