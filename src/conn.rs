use std::net::SocketAddr;
use std::collections::VecDeque;
use std::time::SystemTime;
use crate::{ SERVER_ID, Motd };
use crate::protocol::offline::*;
use binary_utils::*;

pub trait ConnectionAPI {
     fn receive(&mut self, stream: &mut BinaryStream);
     fn gen_motd(&mut self) -> Motd;
}

#[derive(Clone)]
pub struct Connection {
     // read by raknet
     pub send_queue: VecDeque<BinaryStream>,
     pub connected: bool,
     pub address: SocketAddr,
     pub time: SystemTime
}

impl Connection {
     pub fn new(address: SocketAddr, start_time: SystemTime) -> Self {
          Self {
               send_queue: VecDeque::new(),
               connected: false,
               address,
               time: start_time
          }
     }
}

impl ConnectionAPI for Connection {
     /// generic reciever
     fn receive(&mut self, stream: &mut BinaryStream) {
          // They are not connected, perform connection sequence
          if !self.connected {
               let pk = OfflinePackets::recv(stream.read_byte());
               let handler = handle_offline(self, pk, stream);

               self.send_queue.push_back(handler.clone());
          } else {
          }
     }

     fn gen_motd(&mut self) -> Motd {
          Motd {
               name: String::from("Netrex Server"),
               player_count: 10,
               player_max: 100,
               protocol: 420,
               gamemode: String::from("Creative"),
               version: String::from("1.17.0"),
               server_id: SERVER_ID
          }
     }
}