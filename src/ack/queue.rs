use std::collections::HashMap;
use super::{Ack, Record, SingleRecord};
use binary_utils::{BinaryStream};
/// Stores sequence numbers and their relevant data sets.
#[derive(Clone)]
pub struct AckQueue {
     current: u64,
     map: HashMap<u64, BinaryStream>,
     is_ack: bool
}

impl AckQueue {
     pub fn new(is_ack: bool) -> Self {
          Self {
               current: 0,
               map: HashMap::new(),
               is_ack
          }
     }

     pub fn make_ack(&mut self) -> Ack {
          let mut records: Vec<Record> = Vec::new();

          for (seq, _) in self.map.clone().iter() {
               self.drop_seq(*seq);
               records.push(Record::Single(SingleRecord { sequence: *seq as u32 }));
          }

          let mut ack = Ack::new(records.len() as u16, self.is_ack);
          ack.records = records;

          ack
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