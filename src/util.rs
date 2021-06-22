use binary_utils::stream::*;

// Raknet utilities
pub trait IPacketStreamWrite {
     fn write_magic(&mut self);
}

pub trait IPacketStreamRead {
     fn read_magic(&mut self) -> Vec<u8>;
}

impl IPacketStreamWrite for BinaryStream {
     fn write_magic(&mut self) {

     }
}

impl IPacketStreamRead for BinaryStream {
     fn read_magic(&mut self) -> Vec<u8> {
          crate::MAGIC.to_vec()
     }
}