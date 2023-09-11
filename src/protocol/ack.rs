pub const ACK: u8 = 0xc0;
pub const NACK: u8 = 0xa0;

use std::ops::Range;

use binary_util::{
    interfaces::{Reader, Writer},
    types::u24,
    BinaryIo,
};

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
#[derive(Debug, Clone, BinaryIo)]
#[repr(u8)]
pub enum Record {
    Single(SingleRecord) = 1,
    Range(RangeRecord) = 0,
}

#[derive(Debug, Clone)]
pub struct SingleRecord {
    pub sequence: u24,
}

impl Reader<SingleRecord> for SingleRecord {
    fn read(buf: &mut binary_util::ByteReader) -> Result<SingleRecord, std::io::Error> {
        Ok(SingleRecord {
            sequence: buf.read_u24_le()?.into(),
        })
    }
}

impl Writer for SingleRecord {
    fn write(&self, buf: &mut binary_util::ByteWriter) -> Result<(), std::io::Error> {
        buf.write_u24_le(self.sequence)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RangeRecord {
    pub start: u24,
    pub end: u24,
}

impl Reader<RangeRecord> for RangeRecord {
    fn read(buf: &mut binary_util::ByteReader) -> Result<RangeRecord, std::io::Error> {
        Ok(RangeRecord {
            start: buf.read_u24_le()?.into(),
            end: buf.read_u24_le()?.into(),
        })
    }
}

impl Writer for RangeRecord {
    fn write(&self, buf: &mut binary_util::ByteWriter) -> Result<(), std::io::Error> {
        buf.write_u24_le(self.start)?;
        buf.write_u24_le(self.end)?;
        Ok(())
    }
}

#[allow(dead_code)]
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
    pub fn new(count: u16, nack: bool, records: Vec<Record>) -> Self {
        Self {
            id: if nack { 0xa0 } else { 0xc0 },
            count,
            records: records,
        }
    }

    pub fn is_nack(&self) -> bool {
        self.id == 0xa0
    }

    pub fn from_records(mut sequences: Vec<u32>, nack: bool) -> Self {
        // there at least one record
        if sequences.len() > 0 {
            // these sequences may not be in order.
            sequences.sort_unstable();

            // however sometimes we need to create a range for each record if the numbers are in order
            // this is because the client will send a range if the numbers are in order
            let mut ack_records: Vec<Record> = Vec::new();
            let mut current_range: Range<u32> = 0..0;

            for sequence in sequences.iter() {
                // now check if the current range is 0
                if current_range.start == 0 && current_range.end == 0 && *sequence != 0 {
                    // if it is, set the start and end to the current sequence
                    current_range.start = *sequence;
                    current_range.end = *sequence;
                    // skip the rest of the loop
                    continue;
                }

                // first check if the difference is 1
                if *sequence == current_range.end + 1 {
                    current_range.end = *sequence;
                    // skip the rest of the loop
                    continue;
                } else {
                    // the difference is not 1, so we need to add the current range to the ack records
                    // we also need to reset the current range
                    // before we do this we need to verify the range start and end are not 0
                    if current_range.start == current_range.end {
                        // this is a single record
                        ack_records.push(Record::Single(SingleRecord {
                            sequence: current_range.start.into(),
                        }));
                    } else {
                        // this is a range record
                        ack_records.push(Record::Range(RangeRecord {
                            start: current_range.start.into(),
                            end: current_range.end.into(),
                        }));
                    }
                    current_range.start = *sequence;
                    current_range.end = *sequence;
                }
            }

            // verify last range is not equal to end
            if current_range.start == current_range.end {
                // this is a single record
                ack_records.push(Record::Single(SingleRecord {
                    sequence: current_range.start.into(),
                }));
            } else {
                // this is a range record
                ack_records.push(Record::Range(RangeRecord {
                    start: current_range.start.into(),
                    end: current_range.end.into(),
                }));
            }

            Self::new(ack_records.len() as u16, nack, ack_records)
        } else {
            // There is only one record, we can just add it as a single record
            Self::new(0_u16, nack, Vec::new())
        }
    }
}

impl Writer for Ack {
    fn write(&self, buf: &mut binary_util::ByteWriter) -> Result<(), std::io::Error> {
        buf.write_u8(self.id)?;
        buf.write_u16(self.count)?;
        for record in &self.records {
            buf.write(record.write_to_bytes()?.as_slice())?;
        }
        Ok(())
    }
}

impl Reader<Ack> for Ack {
    fn read(buf: &mut binary_util::ByteReader) -> Result<Ack, std::io::Error> {
        let id = buf.read_u8()?;
        let count = buf.read_u16()?;
        let mut records: Vec<Record> = Vec::new();

        for _ in 0..count {
            let record = buf.read_type::<Record>()?;
            records.push(record);
        }

        Ok(Ack { id, count, records })
    }
}
