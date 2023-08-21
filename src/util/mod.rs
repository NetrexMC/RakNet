#![allow(deprecated)]
#[cfg(feature = "async_std")]
use async_std::task::sleep as async_sleep;
use std::net::{SocketAddr, ToSocketAddrs};
use std::{collections::HashMap, time::SystemTime};
#[cfg(feature = "async_tokio")]
use tokio::time::sleep as async_sleep;

pub(crate) mod debug;

#[derive(Debug, Clone)]
pub struct SafeGenerator<T> {
    pub(crate) sequence: T,
}

impl<T> SafeGenerator<T>
where
    T: Default,
{
    pub fn new() -> Self {
        Self {
            sequence: T::default(),
        }
    }
}

macro_rules! impl_gen {
    ($n: ty) => {
        impl SafeGenerator<$n> {
            pub fn next(&mut self) -> $n {
                self.sequence = self.sequence.wrapping_add(1);
                return self.sequence;
            }

            pub fn get(&self) -> $n {
                self.sequence
            }
        }
    };
}

impl_gen!(u8);
impl_gen!(u16);
impl_gen!(u32);
impl_gen!(u64);
impl_gen!(u128);
impl_gen!(usize);

/// This is a fancy wrapper over a HashMap that serves as
/// a time oriented cache, where you can optionally clean up
/// old and un-used values. Key serves as a `packet_id` in
/// rakrs, but this could be used else-where.
///
/// <style>
/// .warning-2 {
///     background: rgba(255,240,76,0.34) !important;
///     padding: 0.75em;
///     border-left: 2px solid #fce811;
///     font-family: "Source Serif 4", NanumBarunGothic, serif;
///  }
///
/// .warning-2 code {
///     background: rgba(211,201,88,0.64) !important;
/// }
///
/// .alert-2 {
///     background: rgba(255, 76, 76, 0.34) !important;
///     padding: 0.75em;
///     border-left: 2px solid #ff4c4c;
///     font-family: "Source Serif 4", NanumBarunGothic, serif;
/// }
///
/// .alert-2 code {
///     background: rgba(255, 76, 76, 0.64) !important;
/// }
///
/// .notice-2 {
///     background: rgba(88, 211, 255, 0.34) !important;
///     padding: 0.75em;
///     border-left: 2px solid #4c96ff;
///     font-family: "Source Serif 4", NanumBarunGothic, serif;
/// }
///
/// .notice-2 code {
///     background: rgba(88, 211, 255, 0.64) !important;
/// }
/// </style>
///
/// <div class="alert-2">
///     <strong>Warning:</strong>
///     <p>
///         This struct will be removed in <code>0.2.0</code> in favor of <code>RecoveryQueue</code>.
///     </p>
/// </div>
///
///
/// Usage example:
/// ```rust
/// use rak_rs::util::CacheStore;
///
/// let mut myStore: CacheStore<u8, Vec<u8>> = CacheStore::new();
/// let myPacket = (0 as u8, vec![0, 0, 0, 1]);
/// myStore.add(myPacket.0, myPacket.1);
/// // Wait a few seconds
/// myStore.flush();
/// ```
#[derive(Debug, Clone)]
#[deprecated(
    since = "0.0.1",
    note = "This is deprecated in favor of `RecoveryQueue<T>`"
)]
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

pub fn to_address_token(remote: SocketAddr) -> String {
    let mut address = remote.ip().to_string();
    address.push_str(":");
    address.push_str(remote.port().to_string().as_str());
    return address;
}

pub fn from_address_token(remote: String) -> SocketAddr {
    let mut parsed = remote
        .to_socket_addrs()
        .expect("Could not parse remote address.");
    SocketAddr::from(parsed.next().unwrap())
}

pub async fn sleep(duration: std::time::Duration) {
    async_sleep(duration).await;
}
