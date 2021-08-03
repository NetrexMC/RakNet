// frame queues are designed to handle split packets,
// and send packets in parts as well.
use crate::protocol::{Packet, IClientBound, IServerBound};
use super::{Frame, FramePacket};
use std::collections::HashMap;
use binary_utils::*;

#[derive(Copy, Clone, Debug)]
pub struct FragmentInfo {
     pub fragment_size: i16,
     pub fragment_id: u16,
     pub fragment_index: i16
}

/// A Fragment recieved from any frame.
/// Fragments can be reassembled by the FragmentQueue.
pub struct Fragment<'a> {
     index: i16,
     buffer: &'a[u8]
}

impl Fragment<'_> {
     pub fn get_index(&self) -> i16 {
          self.index.clone()
     }

     pub fn get_buffer(&self) -> &[u8] {
          &*self.buffer
     }

     pub fn as_stream(&self) -> BinaryStream {
          BinaryStream::init(&self.buffer.to_vec())
     }
}

/// A list of fragments
/// Holds a list of fragments until they are complete.
pub struct FragmentList<'a> {
     pub fragments: Vec<Fragment<'a>>,
     size: i16
}

impl FragmentList<'_> {
     pub fn new() -> Self {
          Self {
               fragments: Vec::new(),
               size: 0
          }
     }

     /// Gets the **wanted** size of fragments
     pub fn get_size(&self) -> i16 {
          self.size.clone()
     }

     /// Gets the **current** size of fragments
     pub fn length(&self) -> usize {
          self.fragments.len()
     }

     /// Returns whether the wanted size is the same as the fragment list length.
     pub fn is_ready(&self) -> bool {
          self.length() == self.get_size() as usize
     }

     /// Sorts all fragments by their index
     pub fn sort(&mut self) {
          self.fragments.sort_by(|a, b| a.get_index().partial_cmp(&b.get_index()).unwrap());
     }
}

/// A fragment queue
/// Handles fragments from incoming packets and resolves an entire Frame packet when compelete.
pub struct FragmentQueue<'a> {
     /// A map of current fragments.
     fragment_table: HashMap<u16, FragmentList<'a>>,
     sequence: u16
}

impl FragmentQueue<'_> {
     pub fn new() -> Self {
          Self {
               fragment_table: HashMap::new(),
               sequence: 0
          }
     }

     /// Assembles a FramePacket from the given fragment index
     /// assuming that all fragments have been sent.
     pub fn assemble_frame(&mut self, index: u16) -> Option<FramePacket> {
          if !self.fragment_table.contains_key(&index) {
               None
          } else {
               let mut framepk = FramePacket::new();
               let frag_list = self.fragment_table.get_mut(&index).unwrap();

               if !frag_list.is_ready() {
                    None
               } else {
                    for frag in frag_list.fragments.iter() {
                         framepk.frames.push(Frame::recv(frag.as_stream()));
                    }

                    // we can now drop the fragment from the table
                    self.fragment_table.remove(&index);

                    Some(framepk)
               }
          }
     }
}