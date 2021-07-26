pub mod reliability;
use crate::{IServerBound};
use binary_utils::*;
use reliability::*;

pub struct FragmentInfo {
     fragment_size: i16,
     fragment_id: u16,
     fragment_index: i16
}

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
               frame.size = stream.read_ushort();
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