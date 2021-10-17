pub mod fragment;
pub mod reliability;

use binary_utils::{self, Streamable, u24::{u24, u24Reader, u24Writer}};
use std::io::{Cursor, Read, Write};
use byteorder::{ReadBytesExt, WriteBytesExt};
use fragment::FragmentInfo;
use reliability::Reliability;

#[derive(Clone, Debug)]
pub struct Frame {
     // This is a triad
     pub sequence: u64,
     /// Only if reliable
     pub reliable_index: Option<u32>,
     /// Only if sequenced
     pub sequence_index: Option<u32>,
     // Order
     pub order_index: Option<u32>,
     pub order_channel: Option<u8>,

     // fragments
     pub fragment_info: Option<FragmentInfo>,

     pub flags: u8,
     pub size: u16,
     pub reliability: Reliability,
     pub body: Vec<u8>,
}

impl Frame {
     /// Creates a dummy frame
     /// You are expected to update the frame yourself
     /// this will only create a fake frame instance.
     pub fn init() -> Self {
          Self {
               sequence: 0,
               reliable_index: None,
               sequence_index: None,
               order_index: None,
               order_channel: None,
               fragment_info: None,
               flags: 0,
               size: 0,
               reliability: Reliability::from_bit(0),
               body: Vec::new(),
          }
     }
}

impl Streamable for Frame {
     fn compose(source: &[u8], position: &mut usize) -> Self {
          let mut stream = Cursor::new(source.to_vec());
          let mut frame: Frame = Frame::init();
          let flags = stream.read_u8().unwrap();

          frame.flags = flags;
          frame.reliability = Reliability::from_bit(flags);

          let fragmented = (flags & 0x10) > 0;
          let bit_length = stream.read_u16().unwrap();

          if Reliability::is_reliable(frame.reliability.to_byte()) {
               frame.reliable_index = Some(stream.read_u24().unwrap().into());
          }

          if Reliability::is_seq(frame.reliability.to_byte()) {
               frame.sequence_index = Some(stream.read_u24().unwrap().into());
          }

          if Reliability::is_ord(frame.reliability.to_byte()) {
               frame.order_index = Some(stream.read_u24().unwrap().into());
               frame.order_channel = Some(stream.read_u8());
          }

          if fragmented {
               frame.fragment_info = Some(FragmentInfo {
                    fragment_size: stream.read_u32().unwrap(),
                    fragment_id: stream.read_u16(),
                    fragment_index: stream.read_i32().unwrap(),
               });
          }

          frame.size = bit_length / 8;

          if stream.len() > (frame.size as usize) {
               // write sized
               let offset = stream.position();
               let inner_buffer = stream[offset..frame.size];
               stream.set_position(offset + frame.size);
               frame.body = inner_buffer.as_vec();
          }

          frame
     }

     fn parse(&self) -> Vec<u8> {
          let mut stream = Cursor::new(Vec::new());
          let mut flags = self.reliability.to_byte() << 5;

          if self.fragment_info.is_some() {
               if self.fragment_info.unwrap().fragment_size > 0 {
                    flags = flags | 0x10;
               }
          }

          stream.write_u8(flags);
          stream.write_u16((self.body.get_length() as u16) * 8);

          if self.reliable_index.is_some() {
               stream.write_u24(self.reliable_index.unwrap());
          }

          if self.sequence_index.is_some() {
               stream.write_u24(self.sequence_index.unwrap());
          }

          if self.order_index.is_some() {
               stream.write_u24(self.order_index.unwrap());
               stream.write_u8(self.order_channel.unwrap());
          }

          if self.fragment_info.is_some() {
               if self.fragment_info.unwrap().fragment_size > 0 {
                    stream.write_int(self.fragment_info.unwrap().fragment_size);
                    stream.write_u16(self.fragment_info.unwrap().fragment_id);
                    stream.write_int(self.fragment_info.unwrap().fragment_index);
               }
          }

          stream.write(&self.body.get_buffer());
          stream
     }
}
#[derive(Debug)]
pub struct FramePacket {
     pub seq: u24,
     pub frames: Vec<Frame>,
}

impl FramePacket {
     pub fn new() -> Self {
          Self {
               seq: 0,
               frames: Vec::new(),
          }
     }
}

impl Streamable for FramePacket {
     fn parse(&self) -> Vec<u8> {
          let mut stream = Vec::new();
          stream.write_u8(0x80);
          stream.write_u24(self.seq.into());

          for f in self.frames.iter() {
               stream.write(&f.to().get_buffer());
          }
          stream
     }

     fn compose(source: &[u8], position: &mut usize) -> Self {
          let mut packet = FramePacket::new();
          let mut stream = Cursor::new(source);
          packet.seq = stream.read_u24().into();

          loop {
               if stream.position() >= source.len() {
                    return packet;
               }

               let offset = stream.position();
               let frm = Frame::recv(stream[offset..]);

               packet.frames.push(frm.clone());
               if frm.write().len() + offset >= source.len() {
                    return packet;
               } else {
                    stream.set_position(frm.write().len());
               }
          }
     }
}
