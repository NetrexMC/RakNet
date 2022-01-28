use std::collections::HashMap;
/// A packet queue, this is used to store packets that are waiting to be sent.
/// This is internal use for Sessions.

#[derive(Debug, Clone)]
pub struct Queue<T> {
    /// Normal priority packet.
    /// This is the default priority.
    normal: Vec<T>,
    /// Lowest priority packet.
    /// This is the lowest priority.
    low: Vec<T>,
    /// Whether or not the queue is frozen.
    pub frozen: bool,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Queue {
            normal: Vec::new(),
            low: Vec::new(),
            frozen: false,
        }
    }

    /// Pushes a packet to the queue.
    /// Note that packets of high priority will be ignored
    pub fn push(&mut self, packet: T, priority: SendPriority) {
        if self.frozen {
            return;
        }
        match priority {
            SendPriority::Normal => self.normal.push(packet),
            SendPriority::Low => self.low.push(packet),
            SendPriority::Immediate => return,
        }
    }

    pub fn flush_low(&mut self) -> Vec<T> {
        let mut low = Vec::new();
        std::mem::swap(&mut low, &mut self.low);
        low
    }

    pub fn flush_normal(&mut self) -> Vec<T> {
        let mut normal = Vec::new();
        std::mem::swap(&mut normal, &mut self.normal);
        normal
    }

    pub fn flush(&mut self) -> Vec<T> {
        let mut normal = self.flush_normal();
        let mut low = self.flush_low();
        normal.append(&mut low);
        return normal;
    }

    pub fn len(self) -> usize {
        self.normal.len() + self.low.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SendPriority {
    /// The packet needs to be sent as fast as possible.
    /// Packets with this priority are sent immediately.
    Immediate,
    /// The packet needs to be sent, but is not as important as High priority.
    /// Packets with this priority will be batched together in frames.
    /// This is the default priority.
    Normal,
    /// The packet being sent does not need to be reliably sent, packets with this priority are sent
    /// last. (Don't use this for MCPE packets)
    Low,
}

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
