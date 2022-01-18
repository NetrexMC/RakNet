/// A packet queue, this is used to store packets that are waiting to be sent.
/// This is internal use for Sessions.

pub struct Queue<T> {
    /// Highest priority packet.
    high: Vec<T>,
    /// Normal priority packet.
    /// This is the default priority.
    normal: Vec<T>,
    /// Lowest priority packet.
    /// This is the lowest priority.
    
}

pub enum QueuePriority {
    /// The packet should be as fast as possible.
    /// These packets are sent immediately.
    High,
    /// The packet needs to be sent, but is not as important as High priority.
    /// Packets with this priority will be batched together in frames.
    /// This is the default priority.
    Normal,
    /// The packet being sent does not need to be reliably sent, packets with this priority are sent
    /// last. (Don't use this for MCPE packets)
    Low,
}