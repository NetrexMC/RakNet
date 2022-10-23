use std::{collections::HashMap, time::SystemTime};

/// This is a fancy wrapper over a HashMap that serves as
/// a time oriented cache, where you can optionally clean up
/// old and un-used values. Key serves as a `packet_id` in
/// rakrs, but this could be used else-where.
///
/// Usage example:
/// ```rust
/// use rakrs::util::CacheStore;
///
/// let mut myStore: CacheStore<u8, Vec<u8>> = CacheStore::new();
/// let myPacket = (0 as u8, vec![0, 0, 0, 1]);
/// myStore.add(myPacket.0, myPacket.1);
/// // Wait a few seconds
/// myStore.flush();
/// ```
#[derive(Debug, Clone)]
pub struct CacheStore<K, V> {
    pub(crate) store: HashMap<K, (SystemTime, Vec<V>)>,
}

impl<K, V> CacheStore<K, V>
where
    K: std::hash::Hash + std::cmp::Eq,
    V: ?Sized + Clone,
{
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    pub fn add(&mut self, sequence: K, buffer: V) {
        let ent = self
            .store
            .entry(sequence)
            .or_insert((SystemTime::now(), Vec::new()));
        ent.1.push(buffer);
    }

    pub fn add_bulk(&mut self, sequence: K, buffers: Vec<V>) {
        let ent = self
            .store
            .entry(sequence)
            .or_insert((SystemTime::now(), Vec::new()));
        ent.1.extend(buffers);
    }

    // clear old packets from the cache and return them
    pub fn flush(&mut self) -> Vec<(K, SystemTime, Vec<V>)> {
        let mut flushed = Vec::new();
        for (sequence, (time, frames)) in self.store.drain() {
            flushed.push((sequence, time, frames));
        }
        return flushed;
    }

    pub fn flush_key(&mut self, key: K) -> Option<(SystemTime, Vec<V>)> {
        self.store.remove(&key)
    }

    pub fn has(&self, key: &K) -> bool {
        self.store.contains_key(key)
    }
}
