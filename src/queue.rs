use crate::{frame::*, fragment::*};
use binary_utils::*;

pub struct PacketQueue<'a> {
     parts: FragmentQueue<'a>,
     ready: Vec<FramePacket>,
     mtu_size: u64,
     seq: u16
}

impl PacketQueue<'_> {
     pub fn queue(&mut self, buf: BinaryStream) {
          if !self.parts.fragment_table.contains_key(&self.seq) {
               let mut list = FragmentList::new();
               self.parts.fragment_table.insert(self.seq, list);
          } else {
               self.parts.fragment_table.get_mut(&self.seq).unwrap().add_stream(buf);
          }
     }
}