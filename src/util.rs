use binary_utils::stream::*;
use binary_utils::{ IBufferRead, IBufferWrite };
use std::net::{ SocketAddr, IpAddr };

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

     }

     fn write_address(&mut self, add: SocketAddr) {
          if add.is_ipv4() {
               self.write_byte(4);
          } else {
               self.write_byte(6);
          }

          let ipst = add.ip().to_string();
          let ipts: Vec<&str> = ipst.split(".").collect();
     }
}

impl IPacketStreamRead for BinaryStream {
     fn read_magic(&mut self) -> Vec<u8> {
          crate::MAGIC.to_vec()
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