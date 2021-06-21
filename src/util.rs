use binary_utils::{ stream::*, IBufferRead, IBufferWrite };

// Raknet utilities
pub trait IPacketStreamWrite {
     fn write_magic(&mut self);
}

impl IPacketStreamWrite for BinaryStream {
     fn write_magic(&mut self) {

     }
}