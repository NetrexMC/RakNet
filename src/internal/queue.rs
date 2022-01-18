/// A packet queue, this is used to store packets that are waiting to be sent.
/// This is internal use for Sessions.

pub struct Queue<T> {
    /// Normal priority packet.
    /// This is the default priority.
    normal: Vec<T>,
    /// Lowest priority packet.
    /// This is the lowest priority.
    low: Vec<T>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Queue {
            normal: Vec::new(),
            low: Vec::new(),
        }
    }

    /// Pushes a packet to the queue.
    /// Note that packets of high priority will be ignored
    pub fn push(&mut self, packet: T, priority: QueuePriority) {
        match priority {
            QueuePriority::Normal => self.normal.push(packet),
            QueuePriority::Low => self.low.push(packet)
        }
    }
}

pub enum QueuePriority {
    /// The packet needs to be sent, but is not as important as High priority.
    /// Packets with this priority will be batched together in frames.
    /// This is the default priority.
    Normal,
    /// The packet being sent does not need to be reliably sent, packets with this priority are sent
    /// last. (Don't use this for MCPE packets)
    Low,
}