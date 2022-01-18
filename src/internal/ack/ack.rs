use std::io::Cursor;

use binary_utils::Streamable;
use byteorder::{ReadBytesExt, WriteBytesExt, BE};

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
            let temp = self.end;
            self.end = self.start;
            self.start = temp;
        }
    }

    /// Returns a currated list of all records within the range.
    pub fn get_records(self) -> Vec<u32> {
        let mut surely_fixed = self.clone();
        surely_fixed.fix();
        let records: Vec<u32> = (surely_fixed.start..surely_fixed.end).collect();
        return records;
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

    pub fn add_record(&mut self, record: Record) {
        self.records.push(record);
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
                    stream.write_u24::<BE>(rec.sequence)?;
                }
                Record::Range(rec) => {
                    stream.push(0);
                    stream.write_u24::<BE>(rec.start)?;
                    stream.write_u24::<BE>(rec.end)?;
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
                    sequence: stream.read_u24::<BE>().unwrap(),
                };

                records.push(Record::Single(record));
            } else {
                let record: RangeRecord = RangeRecord {
                    start: stream.read_u24::<BE>().unwrap(),
                    end: stream.read_u24::<BE>().unwrap(),
                };

                records.push(Record::Range(record));
            }
        }

        *position += stream.position() as usize;

        Ok(Self { count, records, id })
    }
}
