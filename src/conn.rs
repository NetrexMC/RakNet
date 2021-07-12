use std::net::SocketAddr;
use std::collections::VecDeque;
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
     address: SocketAddr,
}

impl Connection {
     pub fn new(address: SocketAddr) -> Self {
          Self {
               send_queue: VecDeque::new(),
               connected: true,
               address
          }
     }
}

impl ConnectionAPI for Connection {
     /// generic reciever
     fn receive(&mut self, stream: &mut BinaryStream) {
          // They are not connected, perform connection sequence
          if !self.connected {
               let pk_id = stream.read_byte();
               let pk = OfflinePackets::recv(pk_id);

               match pk {
                    OfflinePackets::UnconnectedPing => handle_pong(&mut self, &mut stream)
               }
          }


     }

     fn gen_motd(&mut self) -> Motd {
          Motd {
               name: String::from("Netrex Server"),
               player_count: 0,
               player_max: 0,
               protocol: 420,
               gamemode: String::from("Creative"),
               version: String::from("1.17.0"),
               server_id: SERVER_ID
          }
     }
}