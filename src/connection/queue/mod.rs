pub(crate) mod recv;
pub(crate) mod send;

pub use self::recv::*;
pub use self::send::*;

use std::collections::HashMap;

use crate::protocol::frame::FragmentMeta;
use crate::protocol::frame::Frame;
use crate::server::current_epoch;

pub enum NetQueueError<E> {
    /// The insertion failed for any given reason.
    InvalidInsertion,
    /// The insertion failed and the reason is known.
    InvalidInsertionKnown(String),
    /// The `Item` failed to be removed from the queue.
    ItemDeletionFail,
    /// The `Item` is invalid and can not be retrieved.
    InvalidItem,
    /// The queue is empty.
    EmptyQueue,
    /// The error is a custom error.
    Other(E),
}

pub trait NetQueue<Item> {
    /// The `Item` of the queue.
    // type Item = V;

    /// The "key" that each `Item` is stored under
    /// (used for removal)
    type KeyId;

    /// A custom error specifier for NetQueueError
    type Error;

    /// Inserts `Item` into the queue, given the conditions are fulfilled.
    fn insert(&mut self, item: Item) -> Result<Self::KeyId, NetQueueError<Self::Error>>;

    /// Remove an `Item` from the queue by providing an instance of `Self::KeyId`
    fn remove(&mut self, key: Self::KeyId) -> Result<Item, NetQueueError<Self::Error>>;

    /// Retrieves an `Item` from the queue, by reference.
    fn get(&mut self, key: Self::KeyId) -> Result<&Item, NetQueueError<Self::Error>>;

    /// Clears the entire queue.
    fn flush(&mut self) -> Result<Vec<Item>, NetQueueError<Self::Error>>;
}

/// A recovery queue is used to store packets that need to be resent.
/// This is used for sequenced and ordered packets.
#[derive(Debug, Clone)]
pub struct RecoveryQueue<Item> {
    /// The current queue of packets by timestamp
    /// (seq, (packet, timestamp))
    /// todo use the timestamp for round trip time (RTT)
    queue: HashMap<u32, (u64, Item)>,
}

impl<Item> RecoveryQueue<Item> {
    pub fn new() -> Self {
        Self {
            queue: HashMap::new(),
        }
    }
}

impl<Item> NetQueue<Item> for RecoveryQueue<Item> {
    type KeyId = u32;
    type Error = ();

    fn insert(&mut self, item: Item) -> Result<Self::KeyId, NetQueueError<Self::Error>> {
        let index = self.queue.len() as u32;
        self.queue.insert(index, (current_epoch(), item));
        Ok(index)
    }

    fn remove(&mut self, key: Self::KeyId) -> Result<Item, NetQueueError<Self::Error>> {
        if let Some((_, item)) = self.queue.remove(&key) {
            Ok(item)
        } else {
            Err(NetQueueError::ItemDeletionFail)
        }
    }

    fn get(&mut self, key: Self::KeyId) -> Result<&Item, NetQueueError<Self::Error>> {
        if let Some((_, item)) = self.queue.get(&key) {
            Ok(item)
        } else {
            Err(NetQueueError::ItemDeletionFail)
        }
    }

    fn flush(&mut self) -> Result<Vec<Item>, NetQueueError<Self::Error>> {
        let mut items = Vec::new();
        for (_, (_, item)) in self.queue.drain() {
            items.push(item);
        }
        Ok(items)
    }
}

#[derive(Debug, Clone)]
pub struct OrderedQueue<Item: Clone + std::fmt::Debug> {
    /// The current ordered queue channels
    /// Channel, (Highest Index, Ord Index, Item)
    pub queue: HashMap<u32, Item>,
    /// The window for this queue.
    pub window: (u32, u32),
}

impl<Item> OrderedQueue<Item>
where
    Item: Clone + std::fmt::Debug,
{
    pub fn new() -> Self {
        Self {
            queue: HashMap::new(),
            window: (0, 0),
        }
    }

    pub fn insert(&mut self, index: u32, item: Item) -> bool {
        if index < self.window.0 {
            return false;
        }

        if self.queue.contains_key(&index) {
            return false;
        }

        if index >= self.window.1 {
            self.window.1 = index + 1;
        }

        self.queue.insert(index, item);
        true
    }

    pub fn flush(&mut self) -> Vec<Item> {
        let mut items = Vec::<Item>::new();
        while self.queue.contains_key(&self.window.0) {
            if let Some(item) = self.queue.remove(&self.window.0) {
                items.push(item);
            } else {
                break;
            }
            self.window.0 = self.window.0.wrapping_add(1);
        }

        return items;
    }
}

/// An ordered queue is used to Index incoming packets over a channel
/// within a reliable window time.
///
/// Usage:
/// ```rust
/// use rakrs::conn::queue::OrderedQueue;
/// let mut ord_qu: OrderedQueue<Vec<u8>> = OrderedQueue::new();
/// // Insert a packet with the id of "1"
/// ord_qu.insert(vec![0, 1], 1);
/// ord_qu.insert(vec![1, 0], 5);
/// ord_qu.insert(vec![2, 0], 3);
///
/// // Get the packets we still need.
/// let needed: Vec<u32> = ord_qu.flush_missing();
/// assert_eq!(needed, vec![0, 2, 4]);
///
/// // We would in theory, request these packets, but we're going to insert them
/// ord_qu.insert(vec![2, 0, 0, 1], 4);
/// ord_qu.insert(vec![1, 0, 0, 2], 2);
///
/// // Now let's return our packets in order.
/// // Will return a vector of these packets in order by their "id".
/// let ordered: Vec<Vec<u8>> = ord_qu.flush();
/// ```
#[derive(Debug)]
pub struct OrderedQueueOld<T> {
    /// The queue of packets that are in order. Mapped to the time they were received.
    queue: HashMap<u32, T>,
    /// The current starting scope for the queue.
    /// A start scope or "window start" is the range of packets that we are currently allowing.
    /// Older packets will be ignored simply because they are old.
    scope: (u32, u32),
}

impl<T> Clone for OrderedQueueOld<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        OrderedQueueOld {
            queue: self.queue.clone(),
            scope: self.scope.clone(),
        }
    }
}

impl<T> OrderedQueueOld<T>
where
    T: Sized + Clone,
{
    pub fn new() -> Self {
        Self {
            queue: HashMap::new(),
            scope: (0, 0),
        }
    }

    /// Inserts the given packet into the queue.
    /// This will return `false` if the packet is out of scope.
    pub fn insert(&mut self, packet: T, id: u32) -> bool {
        // if the packet id is lower than our scope, ignore it
        // this packet is way to old for us to handle.
        if id < self.scope.0 {
            return false;
        }

        // If the packet is higher than our current scope, we need to adjust our scope.
        // This is because we are now allowing packets that are newer than our current scope.
        if id > self.scope.1 {
            self.scope.1 = id + 1;
        }

        self.queue.insert(id, packet);
        return true;
    }

    /// Drains the current queue by removing all packets from the queue.
    /// This will return the packets in order only if they were within the current scope.
    /// This method will also update the scope and adjust it to the newest window.
    pub fn flush(&mut self) -> Vec<T> {
        // clear all packets not within our scope
        self.clear_out_of_scope();

        // now drain the queue
        let mut map = HashMap::new();
        std::mem::swap(&mut map, &mut self.queue);

        let mut clean = map.iter().collect::<Vec<_>>();
        clean.sort_by_key(|m| m.0);

        return clean.iter().map(|m| m.1.clone()).collect::<Vec<T>>();
    }

    /// Clears all packets that are out of scope.
    /// Returning only the ones that have not been recieved.
    pub fn flush_missing(&mut self) -> Vec<u32> {
        let mut missing: Vec<u32> = Vec::new();
        // we need to get the amount of ids that are missing from the queue.
        for i in self.scope.0..self.scope.1 {
            if !self.queue.contains_key(&i) {
                missing.push(i);
            }
        }

        // we can safely update the scope
        self.scope.0 = missing.get(0).unwrap_or(&self.scope.0).clone();
        return missing;
    }

    fn clear_out_of_scope(&mut self) {
        // clear all packets not within our current scope.
        // this is done by removing all packets that are older than our current scope.
        for (id, _) in self.queue.clone().iter() {
            if *id < self.scope.0 {
                self.queue.remove(id);
            }
        }
    }

    pub fn get_scope(&self) -> u32 {
        self.scope.1 - self.scope.0
    }
}

/// A specialized structure for re-ordering fragments over the wire.
/// You can use this structure to fragment frames as well.
///
/// **NOTE:** This structure will NOT update a frame's reliable index!
/// The sender is required to this!
#[derive(Clone, Debug)]
pub struct FragmentQueue {
    /// The current fragment id to use
    /// If for some reason this wraps back to 0,
    /// and the fragment queue is full, 0 is then cleared and reused.
    fragment_id: u16,

    /// The current Fragments
    /// Hashmap is by Fragment id, with the value being
    /// (`size`, Vec<Frame>)
    fragments: HashMap<u16, (u32, Vec<Frame>)>,
}

impl FragmentQueue {
    pub fn new() -> Self {
        Self {
            fragment_id: 0,
            fragments: HashMap::new(),
        }
    }

    /// Inserts the frame into the fragment queue.
    /// Returns a result tuple of (`fragment_size`, `fragment_index`)
    pub fn insert(&mut self, fragment: Frame) -> Result<(u32, u32), FragmentQueueError> {
        if let Some(meta) = fragment.fragment_meta.clone() {
            if let Some((size, frames)) = self.fragments.get_mut(&meta.id) {
                // check if the frame index is out of bounds
                if meta.index >= *size {
                    return Err(FragmentQueueError::FrameIndexOutOfBounds);
                }
                // the frame exists, and we have parts, check if we have this particular frame already.
                if let Some(_) = frames
                    .iter()
                    .find(|&f| f.fragment_meta.as_ref().unwrap().index == meta.index)
                {
                    // We already have this frame! Do not replace it!!
                    return Err(FragmentQueueError::FrameExists);
                } else {
                    frames.push(fragment);
                    return Ok((meta.size, meta.index));
                }
            } else {
                // We don't already have this fragment index!
                let (size, mut frames) = (meta.size, Vec::<Frame>::new());
                frames.push(fragment);

                self.fragments.insert(meta.id, (size, frames));
                return Ok((meta.size, meta.index));
            }
        }

        return Err(FragmentQueueError::FrameNotFragmented);
    }

    /// Attempts to collect all fragments from a given fragment id.
    /// Will fail if not all fragments are specified.
    pub fn collect(&mut self, id: u16) -> Result<Vec<u8>, FragmentQueueError> {
        if let Some((size, frames)) = self.fragments.get_mut(&id) {
            if *size == frames.len() as u32 {
                // we have all the frames!
                frames.sort_by(|a, b| {
                    a.fragment_meta
                        .as_ref()
                        .unwrap()
                        .id
                        .cmp(&b.fragment_meta.as_ref().unwrap().id)
                });

                let mut buffer = Vec::<u8>::new();

                for frame in frames.iter() {
                    buffer.extend_from_slice(&frame.body);
                }

                self.fragments.remove(&id);
                return Ok(buffer);
            }
            return Err(FragmentQueueError::FragmentsMissing);
        }

        return Err(FragmentQueueError::FragmentInvalid);
    }

    /// This will split a given frame into a bunch of smaller frames within the specified
    /// restriction.
    pub fn split_insert(&mut self, frame: Frame, mtu: u16) -> Result<u16, FragmentQueueError> {
        let max_mtu = mtu - 60;

        if frame.body.len() > max_mtu.into() {
            let splits = frame
                .body
                .chunks(max_mtu.into())
                .map(|c| c.to_vec())
                .collect::<Vec<Vec<u8>>>();
            let id = self.fragment_id.wrapping_add(1);
            let mut index = 0;
            let mut frames: Vec<Frame> = Vec::new();

            for buf in splits.iter() {
                let mut f = Frame::init();
                f.fragment_meta = Some(FragmentMeta {
                    index,
                    size: splits.len() as u32,
                    id,
                });

                f.reliability = frame.reliability;
                f.flags = frame.flags;
                f.size = buf.len() as u16;
                f.body = buf.clone();
                f.order_index = frame.order_index;
                f.order_channel = frame.order_channel;
                f.reliable_index = frame.reliable_index;

                index += 1;

                frames.push(f);
            }

            if self.fragments.contains_key(&id) {
                self.fragments.remove(&id);
            }

            self.fragments.insert(id, (splits.len() as u32, frames));
            return Ok(id);
        }

        return Err(FragmentQueueError::DoesNotNeedSplit);
    }

    pub fn get(&self, id: &u16) -> Result<&(u32, Vec<Frame>), FragmentQueueError> {
        if let Some(v) = self.fragments.get(id) {
            return Ok(v);
        }

        return Err(FragmentQueueError::FragmentInvalid);
    }

    pub fn get_mut(&mut self, id: &u16) -> Result<&mut (u32, Vec<Frame>), FragmentQueueError> {
        if let Some(v) = self.fragments.get_mut(id) {
            return Ok(v);
        }

        return Err(FragmentQueueError::FragmentInvalid);
    }

    pub fn remove(&mut self, id: &u16) -> bool {
        self.fragments.remove(id).is_some()
    }

    /// This will hard clear the fragment queue, this should only be used if memory becomes an issue!
    pub fn clear(&mut self) {
        self.fragment_id = 0;
        self.fragments.clear();
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FragmentQueueError {
    FrameExists,
    FrameNotFragmented,
    DoesNotNeedSplit,
    FragmentInvalid,
    FragmentsMissing,
    FrameIndexOutOfBounds,
}
