pub mod fragment;
pub mod reliability;
use binary_utils::{BinaryStream, IBinaryStream, IBufferWrite, IBufferRead};
use reliability::Reliability;
use reliability::ReliabilityFlag;
use fragment::FragmentInfo;
use crate::{IServerBound, IClientBound};
use crate::conn::*;

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
     pub body: BinaryStream
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
               body: BinaryStream::new()
          }
     }
}

impl IServerBound<Frame> for Frame {
     fn recv(mut stream: BinaryStream) -> Frame {
          let mut frame: Frame = Frame::init();
          let flags = stream.read_byte();

          frame.flags = flags;
          frame.reliability = Reliability::from_bit(flags);

          let fragmented = (flags & 0x10) > 0;
          let bit_length = stream.read_ushort();

          if Reliability::is_reliable(frame.reliability.to_byte()) {
               frame.reliable_index = Some(stream.read_triad());
          }

          if Reliability::is_seq(frame.reliability.to_byte()) {
               frame.sequence_index = Some(stream.read_triad());
          }

          if Reliability::is_ord(frame.reliability.to_byte()) {
               frame.order_index = Some(stream.read_triad());
               frame.order_channel = Some(stream.read_byte());
          }

          if fragmented {
               frame.fragment_info = Some(FragmentInfo {
                    fragment_size: stream.read_int(),
                    fragment_id: stream.read_ushort(),
                    fragment_index: stream.read_int()
               });
          }

          frame.size = bit_length / 8;

          if stream.is_within_bounds(frame.size as usize) {
               let inner_buffer = stream.read_slice(Some(frame.size as usize));
               frame.body = BinaryStream::init(&inner_buffer);
          }

          frame
     }
}

impl IClientBound<Frame> for Frame {
     fn to(&self) -> BinaryStream {
          let mut stream = BinaryStream::new();
          let mut flags = self.reliability.to_byte() << 5;

          if self.fragment_info.is_some() {
               if self.fragment_info.unwrap().fragment_size > 0 {
                    flags = flags | 0x10;
               }
          }

          stream.write_byte(flags);
          stream.write_ushort((self.body.get_length() as u16) * 8);

          if self.reliable_index.is_some() {
               stream.write_triad(self.reliable_index.unwrap());
          }

          if self.sequence_index.is_some() {
               stream.write_triad(self.sequence_index.unwrap());
          }

          if self.order_index.is_some() {
               stream.write_triad(self.order_index.unwrap());
               stream.write_byte(self.order_channel.unwrap());
          }

          if self.fragment_info.is_some() {
               if self.fragment_info.unwrap().fragment_size > 0 {
                    stream.write_int(self.fragment_info.unwrap().fragment_size);
                    stream.write_ushort(self.fragment_info.unwrap().fragment_id);
                    stream.write_int(self.fragment_info.unwrap().fragment_index);
               }
          }

          stream.write_slice(&self.body.get_buffer());
          stream
     }
}

#[derive(Debug)]
pub struct FramePacket {
     pub seq: u32,
     pub frames: Vec<Frame>
}

impl FramePacket {
     pub fn new() -> Self {
          Self {
               seq: 0,
               frames: Vec::new()
          }
     }
}

impl IClientBound<FramePacket> for FramePacket {
     fn to(&self) -> BinaryStream {
          let mut stream = BinaryStream::new();
          stream.write_byte(0x80);
          stream.write_triad(self.seq);

          for f in self.frames.iter() {
               stream.write_slice(&f.to().get_buffer());
          }
          stream
     }
}

impl IServerBound<FramePacket> for FramePacket {
     fn recv(mut stream: BinaryStream) -> FramePacket {
          let mut packet = FramePacket::new();
          stream.read_byte();
          packet.seq = stream.read_triad();

          loop {
               if stream.get_offset() >= stream.get_length() {
                    return packet;
               }

               let offset = stream.get_offset();
               let frm = Frame::recv(stream.slice(offset - 1, None));
               packet.frames.push(frm.clone());
               if frm.to().get_length() + stream.get_offset() >= stream.get_length() {
                    return packet;
               } else {
                    stream.increase_offset(Some(frm.to().get_length()));
               }
          }
     }
}