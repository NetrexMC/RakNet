pub mod queue;
use std::io::{Read, Write};
use binary_utils::*;
use byteorder::{ReadBytesExt, WriteBytesExt};

#[derive(Debug, Clone)]
pub enum Record {
     Single(SingleRecord),
     Range(RangeRecord),
}

impl Record {
     pub fn is_single(&self) -> bool {
          match *self {
               Self::Range(_) => false,
               Self::Single(_) => true,
          }
     }
}

#[derive(Debug, Clone)]
pub struct SingleRecord {
     pub sequence: u32,
}

#[derive(Debug, Clone)]
pub struct RangeRecord {
     pub start: u32,
     pub end: u32,
}

impl RangeRecord {
     /// Gets the sequences in the range of [start, end).
     pub fn get_sequences(&self) -> Vec<u32> {
          let mut seqs = vec![];
          let highest = if self.end > self.start {
               self.end
          } else {
               self.start
          };
          let lowest = if self.end > self.start {
               self.start
          } else {
               self.end
          };
          for i in lowest..highest {
               seqs.push(i);
          }
          seqs
     }
}

#[derive(Debug, Clone)]
pub struct Ack {
     pub count: u16,
     pub records: Vec<Record>,
     pub id: AckIds,
}

impl Ack {
     pub fn new(count: u16, nack: bool) -> Self {
          Self {
               count,
               records: Vec::new(),
               id: match nack {
                    true => AckIds::Acknowledge,
                    false => AckIds::NoAcknowledge,
               },
          }
     }
}

impl Streamable for Ack {
     fn compose(source: &[u8], position: &mut usize) -> Self {
          let mut stream = Vec::<u8>::new();
          let id = stream.read_u8();
          let count = stream.read_u16();
          let mut records: Vec<Record> = Vec::new();
          for _ in 0..count {
               if stream.read_bool() {
                    let record: SingleRecord = SingleRecord {
                         sequence: stream.read_u24().into(),
                    };

                    records.push(Record::Single(record));
               } else {
                    let record: RangeRecord = RangeRecord {
                         start: stream.read_u24().into(),
                         end: stream.read_u24().into(),
                    };

                    records.push(Record::Range(record));
               }
          }

          Self {
               count,
               records,
               id: AckIds::from_byte(id),
          }
     }

     fn parse(&self) -> Vec<u8> {
          let mut stream = Vec::<u8>::new();
          stream.write_u8(AckIds::Acknowledge as u8);
          stream.write_u16(self.count);

          for record in self.records.iter() {
               match record {
                    Record::Single(rec) => {
                         stream.write_u8(true.into());
                         stream.write_u24(rec.sequence);
                    }
                    Record::Range(rec) => {
                         stream.write_u8(false.into());
                         stream.write_u24(rec.start);
                         stream.write_u24(rec.end);
                    }
               }
          }
          stream
     }
}

#[derive(Debug, Clone)]
pub enum AckIds {
     Acknowledge = 0xc0,
     NoAcknowledge = 0xa0,
}

impl AckIds {
     pub fn from_byte(byte: u8) -> Self {
          match byte {
               0xa0 => Self::NoAcknowledge,
               _ => Self::Acknowledge,
          }
     }

     pub fn to_byte(&self) -> u8 {
          match *self {
               Self::Acknowledge => 0xc0,
               Self::NoAcknowledge => 0xa0,
          }
     }
}

pub fn is_ack_or_nack(byte: u8) -> bool {
     byte == 0xa0 || byte == 0xc0
}
