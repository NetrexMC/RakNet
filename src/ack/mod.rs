pub mod queue;
use crate::{IClientBound, IServerBound};
use binary_utils::*;

#[derive(Debug, Clone)]
pub enum Record {
     Single(SingleRecord),
     Range(RangeRecord)
}

#[derive(Debug, Clone)]
pub struct SingleRecord {
     pub sequence: u32
}

#[derive(Debug, Clone)]
pub struct RangeRecord {
     pub start: u32,
     pub end: u32
}

#[derive(Debug, Clone)]
pub struct Ack {
     pub count: u16,
     pub record: Record
}

impl Ack {
     pub fn new(count: u16, record: Record) -> Self {
          Self {
               count,
               record
          }
     }
}

pub struct NAck {
     pub count: u16,
     pub record: Record
}

impl NAck {
     pub fn new(count: u16, record: Record) -> Self {
          Self {
               count,
               record
          }
     }
}


pub enum AckIds {
     Acknowledge = 0xc0,
     NoAcknowledge = 0xa0
}

pub fn is_ack_or_nack(byte: u8) -> bool {
     byte == 0xa0 || byte == 0xc0
}

impl IServerBound<Ack> for Ack {
     fn recv(mut stream: BinaryStream) -> Self {
          let count = stream.read_ushort();
          if stream.read_bool() {
               Self {
                    count,
                    record: Record::Single(SingleRecord {
                         sequence: stream.read_triad()
                    })
               }
          } else {
               Self {
                    count,
                    record: Record::Range(RangeRecord {
                         start: stream.read_triad(),
                         end: stream.read_triad()
                    })
               }
          }
     }
}

impl IClientBound<Ack> for Ack {
     fn to(&self) -> BinaryStream {
          let mut stream = BinaryStream::new();
          stream.write_byte(AckIds::Acknowledge as u8);

          match &(*self).record {
               Record::Range(record) => {
                    stream.write_bool(false);
                    stream.write_triad(record.start);
                    stream.write_triad(record.end);
               },
               Record::Single(record) => {
                    stream.write_bool(true);
                    stream.write_triad(record.sequence);
               }
          }
          stream
     }
}