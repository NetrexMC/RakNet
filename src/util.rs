use binary_utils::stream::*;
use binary_utils::{ IBufferRead, IBufferWrite };
use std::net::{ SocketAddr, IpAddr };
use std::marker::Sync;
use crate::MAGIC;
use crate::conn::Connection;

// Raknet utilities
pub trait IPacketStreamWrite {
     fn write_magic(&mut self);

     fn write_address(&mut self, add: SocketAddr);
}

pub trait IPacketStreamRead {
     fn read_magic(&mut self) -> Vec<u8>;

     fn read_address(&mut self) -> SocketAddr;
}

impl IPacketStreamWrite for BinaryStream {
     fn write_magic(&mut self) {
          self.write_slice(&MAGIC);
     }

     fn write_address(&mut self, add: SocketAddr) {
          if add.is_ipv4() {
               self.write_byte(4);
          } else {
               self.write_byte(6);
          }

          let ipst = add.ip().to_string();
          let ipts: Vec<&str> = ipst.split(".").collect();

          for p in ipts {
               let byte = u16::from_str_radix(p, 10).unwrap();
               self.write_byte(byte);
          }
          self.write_byte(add.port());
     }
}

impl IPacketStreamRead for BinaryStream {
     fn read_magic(&mut self) -> Vec<u8> {
          self.read_slice(Some(16))
     }

     fn read_address(&mut self) -> SocketAddr {
          let addr_type = self.read_byte();
          if addr_type == 4 {
               let parts = self.read_slice(Some(4 as usize));
               let port = self.read_short();
               SocketAddr::new(IpAddr::from([parts[0], parts[1], parts[2], parts[3]]), port)
          } else {
               SocketAddr::new(IpAddr::from([0,0,0,0]), 0)
          }
     }
}

/// Events
/// Events are READ ONLY and can only be COPIED
pub enum RakEv {
     ClientConnect(Connection),
     KillServer(bool),
     Recieve(SocketAddr, BinaryStream),
     Send(SocketAddr, BinaryStream)
}

pub type RakEvClosure = Box<dyn Fn(&RakEv) + Send + Sync>;

pub trait IRakEmit {
     fn new() -> Self;
     fn register(&mut self, listener: RakEvClosure) -> bool;

     /// Reserved for raknet
     fn broadcast(&self, event: &RakEv);
}

pub struct RakEmitter {
     listeners: Vec<RakEvClosure>,
     max: u8
}

impl IRakEmit for RakEmitter {
     fn new() -> Self {
          Self {
               listeners: Vec::new(),
               max: 5
          }
     }

     fn register(&mut self, listener: RakEvClosure) -> bool {
          self.listeners.push(listener);
          return true
     }

     fn broadcast(&self, ev: &RakEv) {
          for listener in &self.listeners {
               listener(&ev);
          }
     }
}
