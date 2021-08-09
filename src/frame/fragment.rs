// frame queues are designed to handle split packets,
// and send packets in parts as well.
use crate::protocol::{Packet, IClientBound, IServerBound};
use super::{Frame, FramePacket, Reliability, ReliabilityFlag};
use std::collections::HashMap;
use binary_utils::*;

#[derive(Copy, Clone, Debug)]
pub struct FragmentInfo {
     pub fragment_size: i16,
     pub fragment_id: u16,
     pub fragment_index: i16
}

impl FragmentInfo {
     pub fn new(fragment_size: i16, fragment_id: u16, fragment_index: i16) -> Self {
          Self {
               fragment_size,
               fragment_id,
               fragment_index
          }
     }
}

/// A Fragment recieved from any frame.
/// Fragments can be reassembled by the FragmentQueue.
#[derive(Clone, Debug)]
pub struct Fragment {
     index: i16,
     buffer: Vec<u8>
}

impl Fragment {
     pub fn new(index: i16, buffer: Vec<u8>) -> Self {
          Self {
               index,
               buffer
          }
     }
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
#[derive(Clone, Debug)]
pub struct FragmentList {
     pub fragments: Vec<Fragment>,
     size: u64
}

impl FragmentList {
     pub fn new() -> Self {
          Self {
               fragments: Vec::new(),
               size: 0
          }
     }

     /// Adds a binary stream to the fragment list.
     pub fn add_stream(&mut self, buf: BinaryStream) {
          // create a fragment from a stream
          let frag = Fragment {
               buffer: buf.get_buffer(),
               index: self.fragments.len() as i16
          };

          self.fragments.push(frag.clone());
     }

     pub fn add_fragment(&mut self, frag: Fragment) {
          self.fragments.push(frag.clone());
          self.size += 1;
     }

     pub fn assemble(&mut self, mtu_size: i16, usable_id: u16) -> Option<Vec<FramePacket>> {
          let mut framepks = Vec::new();
          let mut framepk = FramePacket::new();
          // sort the frames
          self.sort();

          if !self.is_ready() {
               None
          } else {
               let mut index = 0;
               for frag in self.fragments.iter() {
                    let mut frame = Frame::init();
                    frame.fragment_info = Some(FragmentInfo::new(self.size as i16, usable_id, index));
                    frame.body = frag.as_stream();

                    if framepk.to().get_length() + frame.to().get_length() >= mtu_size as usize {
                         framepks.push(framepk);
                         framepk = FramePacket::new();
                    }

                    framepk.frames.push(frame);
                    index += 1;
               }

               // we can now drop the fragment from the table

               Some(framepks)
          }
     }

     /// Gets the **wanted** size of fragments
     pub fn get_size(&self) -> u64 {
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

/// Stores fragmented frames by their frame index.
/// To visualize this:
/// - [frame_index](FragmentInfo#frame_index) -> FragmentList
///
/// **Note:**
/// This is only used if a frame is fragmented to begin with, otherwise it should be ignored.
#[derive(Clone, Debug)]
pub struct FragmentStore {
     /// A map of current fragments.
     pub fragment_table: HashMap<u16, FragmentList>,
     sequence: u16
}

impl FragmentStore {
     pub fn new() -> Self {
          FragmentStore {
               fragment_table: HashMap::new(),
               sequence: 0
          }
     }

     /// Adds a stream into it's given sequence.
     /// Do note, this does not make them frames.
     pub fn add_stream(&mut self, buf: BinaryStream) {
          if !self.fragment_table.contains_key(&self.sequence) {
               let mut list = FragmentList::new();
               self.fragment_table.insert(self.sequence, list);
          } else {
               self.fragment_table.get_mut(&self.sequence).unwrap().add_stream(buf);
          }
     }

     pub fn add_frame(&mut self, frame: Frame) {
          if !self.fragment_table.contains_key(&frame.fragment_info.unwrap().fragment_id) {
               let mut list = FragmentList::new();
               list.add_fragment(Fragment {
                    index: frame.fragment_info.unwrap().fragment_index,
                    buffer: frame.body.get_buffer()
               });
               list.size = frame.fragment_info.unwrap().fragment_size as u64;
               self.fragment_table.insert(frame.fragment_info.unwrap().fragment_id, list);
          } else {
               self.fragment_table.get_mut(&frame.fragment_info.unwrap().fragment_id).unwrap().add_fragment(Fragment {
                    index: frame.fragment_info.unwrap().fragment_index,
                    buffer: frame.body.get_buffer()
               });
          }
     }

     pub fn ready(&mut self, index: u16) -> bool {
          if self.fragment_table.contains_key(&index) {
               let fragment_list = self.fragment_table.get_mut(&index).unwrap();
               return fragment_list.fragments.len() as u64 == fragment_list.size;
          } else {
               return false;
          }
     }

     /// Assembles a FramePacket from the given fragment index
     /// assuming that all fragments have been sent.
     pub fn assemble_frame(&mut self, index: u16, size: i16, usable_id: u16) -> Option<FramePacket> {
          if !self.fragment_table.contains_key(&index) {
               None
          } else {
               let assembly = self.fragment_table.get_mut(&index).unwrap().assemble(size, usable_id);
               let mut frame_pk = FramePacket::new();

               if assembly.is_some() {
                    self.fragment_table.remove(&index);

                    for fpk in assembly.unwrap().into_iter() {
                         for frame in fpk.frames {
                              frame_pk.frames.push(frame);
                         }
                    }

                    Some(frame_pk)
               } else {
                    None
               }
          }
     }
}