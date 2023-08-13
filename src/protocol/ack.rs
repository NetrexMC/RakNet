pub const ACK: u8 = 0xc0;
pub const NACK: u8 = 0xa0;

use std::ops::Range;

use binary_util::{BinaryIo, types::{u24, LE}};

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

#[derive(Debug, Clone, BinaryIo)]
pub struct SingleRecord {
    pub sequence: LE<u24>,
}

#[derive(Debug, Clone, BinaryIo)]
pub struct RangeRecord {
    pub start: LE<u24>,
    pub end: LE<u24>,
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

#[derive(Debug, Clone, BinaryIo)]
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

    pub fn from_records(mut missing: Vec<u32>, nack: bool) -> Self {
        let mut records: Vec<Record> = Vec::new();
        let mut current: Range<u32> = 0..0;
        missing.sort();

        for m in missing.clone() {
            if current.start == 0 {
                current.start = m;
                current.end = m;
            } else if m == current.end + 1 {
                current.end = m;
            } else {
                // end of range
                if current.start == current.end {
                    records.push(
                        Record::Single(
                            SingleRecord {
                                sequence: LE(current.start.into()),
                            }
                        )
                    );
                } else {
                    records.push(
                        Record::Range(
                            RangeRecord {
                                start: LE(current.start.into()),
                                end: LE(current.end.into()),
                            }
                        )
                    );
                }

                current.start = m;
                current.end = m;
            }
        }

        let mut nack = Self::new(records.len().try_into().unwrap(), nack);
        nack.records = records;

        return nack;
    }
}
