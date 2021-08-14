use std::net::SocketAddr;
use std::time::SystemTime;
use std::sync::Arc;
use crate::{Motd};
use binary_utils::*;

pub type RecievePacketFn = fn(&mut Connection, &mut BinaryStream);

pub trait ConnectionAPI {
     /// Called when a packet is recieved from raknet
     /// This is called on each **Frame**
     fn recieve_packet(&mut self, stream: &mut BinaryStream);

     // / Called when RakNet wants to generate a **Motd**
     // / for the server, if this fails, the `default_motd`
     // / function is called instead.
     // fn gen_motd(&mut self) -> Motd;
}

#[derive(Clone, PartialEq)]
pub enum ConnectionState {
     Connecting,
     Connected,
     Disconnected,
     Offline
}

impl ConnectionState {
     pub fn is_disconnected(&self) -> bool {
          match *self {
               Self::Disconnected => true,
               _ => false
          }
     }
     pub fn is_available(&self) -> bool {
          match *self {
               Self::Disconnected => false,
               _ => true
          }
     }
}

#[derive(Clone)]
pub struct Connection {
     pub address: SocketAddr,
     pub time: SystemTime,
     pub motd: Motd,
     pub mtu_size: u16,
     pub state: ConnectionState,
     pub recv: Arc<RecievePacketFn>
}

impl Connection {
     pub fn new(address: SocketAddr, start_time: SystemTime, recv: Arc<RecievePacketFn>) -> Self {
          Self {
               address,
               time: start_time,
               motd: Motd::default(),
               mtu_size: 0,
               state: ConnectionState::Disconnected,
               recv
          }
     }
}