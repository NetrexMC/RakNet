pub mod queue;
use crate::{IClientBound, IServerBound};
use binary_utils::*;

#[derive(Debug, Clone)]
pub enum Record {
     Single(SingleRecord),
     Range(RangeRecord)
}

impl Record {
     pub fn is_single(&self) -> bool {
          match *self {
               Self::Range(_) => false,
               Self::Single(_) => true
          }
     }
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
     pub records: Vec<Record>,
     pub id: AckIds
}

impl Ack {
     pub fn new(count: u16, nack: bool) -> Self {
          Self {
               count,
               records: Vec::new(),
               id: match nack {
                    true => AckIds::Acknowledge,
                    false => AckIds::NoAcknowledge
               }
          }
     }
}


impl IServerBound<Ack> for Ack {
     fn recv(mut stream: BinaryStream) -> Self {
          let id = stream.read_byte();
          let count = stream.read_ushort();
          let mut records: Vec<Record> = Vec::new();
          for _ in 0..count {
               if stream.read_bool() {
                    let record: SingleRecord = SingleRecord {
                         sequence: stream.read_triad()
                    };

                    records.push(Record::Single(record));
               } else {
                    let record: RangeRecord = RangeRecord {
                         start: stream.read_triad(),
                         end: stream.read_triad()
                    };

                    records.push(Record::Range(record));
               }
          }

          Self {
               count,
               records,
               id: AckIds::from_byte(id)
          }
     }
}

impl IClientBound<Ack> for Ack {
     fn to(&self) -> BinaryStream {
          let mut stream = BinaryStream::new();
          stream.write_byte(AckIds::Acknowledge as u8);

          for record in self.records.iter() {
               match record {
                    Record::Single(rec) => {
                         stream.write_bool(true);
                         stream.write_triad(rec.sequence);
                    },
                    Record::Range(rec) => {
                         stream.write_bool(false);
                         stream.write_triad(rec.start);
                         stream.write_triad(rec.end);
                    }
               }
          }
          stream
     }
}

#[derive(Debug, Clone)]
pub enum AckIds {
     Acknowledge = 0xc0,
     NoAcknowledge = 0xa0
}

impl AckIds {
     pub fn from_byte(byte: u8) -> Self {
          match byte {
               0xa0 => Self::NoAcknowledge,
               _ => Self::Acknowledge
          }
     }

     pub fn to_byte(&self) -> u8 {
          match *self {
               Self::Acknowledge => 0xc0,
               Self::NoAcknowledge => 0xa0
          }
     }
}

pub fn is_ack_or_nack(byte: u8) -> bool {
     byte == 0xa0 || byte == 0xc0
}