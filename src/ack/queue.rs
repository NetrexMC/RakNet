use std::collections::HashMap;
use super::{Ack, Record, SingleRecord, RangeRecord};
use binary_utils::{BinaryStream, IBinaryStream};
/// Stores sequence numbers and their relevant data sets.
#[derive(Clone)]
pub struct AckQueue {
     current: u64,
     map: HashMap<u64, BinaryStream>
}

impl AckQueue {
     pub fn new() -> Self {
          Self {
               current: 0,
               map: HashMap::new()
          }
     }

     pub fn make_ack(&mut self) -> Ack {
          let mut first = 0;
          for (k, _v) in self.map.iter() {
               if first == 0 {
                    first = *k;
               }
          }
          let record = RangeRecord {
               start: first as u32,
               end: self.map.len() as u32
          };

          for x in record.start..record.end {
               self.drop_seq(x as u64);
          }

          Ack::new(((record.start + record.end) as u16) / 2, Record::Range(record))
     }

     pub fn increment_seq(&mut self, by: Option<u64>) {
          self.current += by.unwrap_or(1);
     }

     pub fn push_seq(&mut self, idx: u64, val: BinaryStream) {
          self.map.insert(idx, val);
     }

     pub fn drop_seq(&mut self, idx: u64) -> bool {
          if self.map.contains_key(&idx) {
               self.map.remove_entry(&idx);
               true
          } else {
               false
          }
     }

     pub fn get_seq(&self, idx: u64) -> Option<&BinaryStream> {
          if self.map.contains_key(&idx) {
               self.map.get(&idx)
          } else {
               None
          }
     }

     pub fn has_seq(&self, idx: u64) -> bool {
          self.map.contains_key(&idx)
     }

     pub fn is_empty(&self) -> bool {
          self.map.len() == 0
     }
}