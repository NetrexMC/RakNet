use std::{collections::HashMap, time::SystemTime};

// this is a cache store for packets,
// any packets in here will be ticked, and over time, will be resent, or discarded
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
