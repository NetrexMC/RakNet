pub const ACK: u8 = 0xc0;
pub const NACK: u8 = 0xa0;

use std::{io::Cursor, ops::Range};

use binary_utils::Streamable;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt, BE};

pub(crate) trait Ackable {
    type NackItem;

    /// When an ack packet is recieved.
    /// We should ack the queue
    fn ack(&mut self, _: Ack) {}

    /// When an NACK packet is recieved.
    /// We should nack the queue
    /// This should return the packets that need to be resent.
    fn nack(&mut self, _: Ack) -> Vec<Self::NackItem> {
        todo!()
    }
}

/// An ack record.
/// A record holds a single or range of acked packets.
/// No real complexity other than that.
#[derive(Debug, Clone)]
pub enum Record {
    Single(SingleRecord),
    Range(RangeRecord),
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
    /// Fixes the end of the range if it is lower than the start.
    pub fn fix(&mut self) {
        if self.end < self.start {
            std::mem::swap(&mut self.start, &mut self.end);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ack {
    pub id: u8,
    pub count: u16,
    pub records: Vec<Record>,
}

impl Ack {
    pub fn new(count: u16, nack: bool) -> Self {
        Self {
            id: if nack { 0xa0 } else { 0xc0 },
            count,
            records: Vec::new(),
        }
    }

    pub fn is_nack(&self) -> bool {
        self.id == 0xa0
    }

    pub fn from_records(missing: Vec<u32>, nack: bool) -> Self {
        let mut records: Vec<Record> = Vec::new();
        let mut current: Range<u32> = 0..0;

        for m in missing {
            if current.end + 1 == m {
                current.end += 1;
            } else if m > current.end {
                // This is a new range.
                let mut record = RangeRecord {
                    start: current.start,
                    end: current.end,
                };
                record.fix();
                records.push(Record::Range(record));
                current.start = m;
                current.end = m;
            } else {
                // This is a new single.
                records.push(Record::Single(SingleRecord { sequence: m }));
                current.start = m + 1;
                current.end = m + 1;
            }
        }

        let mut nack = Self::new(records.len().try_into().unwrap(), nack);
        nack.records = records;

        return nack;
    }
}

impl Streamable for Ack {
    fn parse(&self) -> Result<Vec<u8>, binary_utils::error::BinaryError> {
        let mut stream: Vec<u8> = Vec::new();
        stream.push(self.id);
        stream.write_u16::<BE>(self.count)?;

        for record in self.records.iter() {
            match record {
                Record::Single(rec) => {
                    stream.push(1);
                    stream.write_u24::<LittleEndian>(rec.sequence)?;
                }
                Record::Range(rec) => {
                    stream.push(0);
                    stream.write_u24::<LittleEndian>(rec.start)?;
                    stream.write_u24::<LittleEndian>(rec.end)?;
                }
            }
        }
        Ok(stream)
    }

    fn compose(
        source: &[u8],
        position: &mut usize,
    ) -> Result<Self, binary_utils::error::BinaryError> {
        let mut stream = Cursor::new(source);
        let id = stream.read_u8().unwrap();
        let count = stream.read_u16::<BE>().unwrap();
        let mut records: Vec<Record> = Vec::new();
        for _ in 0..count {
            if stream.read_u8().unwrap() == 1 {
                let record: SingleRecord = SingleRecord {
                    sequence: stream.read_u24::<LittleEndian>().unwrap(),
                };

                records.push(Record::Single(record));
            } else {
                let mut record: RangeRecord = RangeRecord {
                    start: stream.read_u24::<LittleEndian>().unwrap(),
                    end: stream.read_u24::<LittleEndian>().unwrap(),
                };

                record.fix();

                records.push(Record::Range(record));
            }
        }

        *position += stream.position() as usize;

        Ok(Self { count, records, id })
    }
}
