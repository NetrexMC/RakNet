use std::net::SocketAddr;
use std::time::SystemTime;
use crate::{Motd};
use crate::frame::{fragment::*, FramePacket, Frame};
use crate::handler::{PacketHandler};
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
     pub connected: bool,
     pub address: SocketAddr,
     pub time: SystemTime,
     pub motd: Motd,
     pub mtu_size: u16
}

impl Connection {
     pub fn new(address: SocketAddr, start_time: SystemTime) -> Self {
          Self {
               connected: false,
               address,
               time: start_time,
               motd: Motd::default(),
               mtu_size: 0
          }
     }
}