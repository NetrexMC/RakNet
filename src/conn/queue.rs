use std::collections::HashMap;
use std::collections::VecDeque;

pub enum NetQueueError<E> {
    /// The insertion failed for any given reason.
    InvalidInsertion,
    /// The insertion failed and the reason is known.
    InvalidInsertionKnown(String),
    /// The `Item` failed to be removed from the queue.
    ItemDeletionFail,
    Other(E)
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

/// A specialized struct that will keep records of `T`
/// up to a certain capacity specified with `RecoveryQueue::with_capacity(u32)`
/// during construction. This means any records that are hold, are dropped off and forgotten.
///
/// By default the recovery queue
/// will store `255` records of `T`.
///
/// The maximum records allowed are `u32::MAX`, however not
/// advised.
///
/// ```rust
/// use rakrs::conn::queue::RecoveryQueue;
///
/// // Create a new recovery queue, of u8
/// let mut queue = RecoveryQueue::<u8>::new();
/// let indexes = (
///     // 0
///     queue.insert(1),
///     // 1
///     queue.insert(4),
///     // 2
///     queue.insert(6)
/// );
///
/// queue.recover(1); // Result<0>
/// queue.recover(2); // Result<6>
/// queue.get(1); // Result<4>
///
/// assert_eq!(queue.recover(1), Ok(4));
/// assert_eq!(queue.get(1), Ok(4));
/// assert_eq!(queue.get(4), Err());
/// ```
#[derive(Debug, Clone)]
pub struct RecoveryQueue<Item> {
    /// (index, resend_attempts, Item)
    recovery: VecDeque<(u32, Item)>,
    capacity: u32,
    index: u32,
}

impl<Item> RecoveryQueue<Item> {
    pub fn new() -> Self {
        Self {
            recovery: VecDeque::with_capacity(255),
            capacity: 255,
            index: 0
        }
    }

    pub fn with_capacity(capacity: u32) -> Self {
        Self {
            recovery: VecDeque::with_capacity(capacity.try_into().unwrap()),
            capacity,
            index: 0
        }
    }

    /// Set the capacity of the recovery queue.
    /// This may be called by raknet if a load of clients
    /// start trying to connect or if the I/O of network
    /// begins writing more than reading.
    pub fn set_capacity(&mut self, factor: u32) -> bool {
        // todo: IF factor is greator than self.capacity, replace
        // todo: the current capacity with a vector of that capacity.
        if factor > 0 {
            self.capacity = factor;
            self.validate_capacity();
            true
        } else {
            false
        }
    }

    /// Add a new item into the recovery queue.
    /// If the item addition exceeds the current
    /// capacity of the queue, the queue is shifted.
    ///
    /// This method does not validate for duplicates,
    /// for that, use `new_insert`
    pub fn insert(&mut self, item: Item) -> u32 {
        self.validate_capacity();

        let idx = self.index;
        self.recovery.push_back((idx, item));
        self.index += 1;

        return idx;
    }

    /// Validates that adding a new entry will not exceed
    /// the capacity of the queue itself.
    fn validate_capacity(&mut self) {
        while self.recovery.len() >= self.capacity as usize {
            self.recovery.pop_front();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum RecoveryQueueError {
    /// The index given is not valid, either because it is from the
    /// future, or overflows.
    Invalid,
    /// The index given is not recoverable, but was cached earlier,
    /// you should not try to retrieve this index again.
    IndexOld,
    /// The insertion failed because the Item is already recoverable.
    ///
    /// **This is only enforced if used with `insert_new`**
    Duplicate
}

/// A Record of `Item` where each `Item` may only live for `max_age`.
/// If an `Item` exceeds the duration of `max_age`, it is dropped.
///
/// This structure is **NOT** ticked, meaning any records are stale
/// until the structure is interacted with.
#[derive(Clone, Debug)]
pub struct TimedRecoveryQueue<Item> {
    /// The maximum age a packet is allowed to live in the queue for.
    /// If the age is exceeded, the item will be dropped.
    pub max_age: u32,
    /// A private recovery queue, this will hold our (`Time`, Item)
    /// We will then be able to clear out old packets by "Time", or if
    /// the capacity of the queue is reached.
    queue: RecoveryQueue<(u32, Item)>
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
pub struct OrderedQueue<T> {
    /// The queue of packets that are in order. Mapped to the time they were received.
    queue: HashMap<u32, T>,
    /// The current starting scope for the queue.
    /// A start scope or "window start" is the range of packets that we are currently allowing.
    /// Older packets will be ignored simply because they are old.
    scope: (u32, u32),
}

impl<T> Clone for OrderedQueue<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        OrderedQueue {
            queue: self.queue.clone(),
            scope: self.scope.clone(),
        }
    }
}

impl<T> OrderedQueue<T>
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

/// This queue is used to prioritize packets being sent out
/// Packets that are old, are either dropped or requested again.
/// You can define this behavior with the `timeout` property.
#[derive(Debug, Clone)]
pub struct SendQueue {
    /// The amount of time that needs to pass for a packet to be
    /// dropped or requested again.
    timeout: u16,

    /// The amount of times we should retry sending a packet before
    /// dropping it from the queue. This is currently set to `5`.
    max_tries: u16,

    /// The current sequence number. This is incremented every time
    /// a packet is sent reliably. We can resend these if they are
    /// Acked.
    send_seq: u32,

    /// The reliable packet recovery queue.
    /// This represents a "Sequence Index". Any socket can request
    /// a sequence to resend via ack. These are saved here.
    reliable_queue: TimedRecoveryQueue<Vec<u8>>,

    /// The unreliable packet sequence count.
    unreliable_index: u32,

    /// This is a special queue nested within the send queue. It will
    /// automatically clean up packets that "are out of scope" or
    /// "outside the window"
    ord_queue: OrderedQueue<Vec<u8>>
    
}

impl SendQueue {

}

#[derive(Debug, Clone)]
pub struct RecvQueue {}
