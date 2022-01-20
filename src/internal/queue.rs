use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum QueueError {
    /// The queue is full
    Full,
    /// The queue failed to flush
    Flush,
    /// The queue failed to parition
    Partition,
}

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueError::Full => write!(f, "The queue is full"),
            QueueError::Flush => write!(f, "The queue failed to flush"),
            QueueError::Partition => write!(f, "The queue failed to partition"),
        }
    }
}

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

/// Buffered Queues, this is used internally for ordered channels and fragments.
pub struct BufferQueue<T> {
    /// The total number of packets in this queue.
    /// By default, we do not allow more than 256 packets in a queue.
    pub size: u32,
    /// The maximum size for each buffer in the queue.
    /// By default this is 1400 - 4 - 2 - 4 - 1 - 3 - 3 - 3 - 2 - 1 - 3 - 1
    /// Which is: 1373
    /// However you should not set this to more than the MtuSize of the connection.
    pub partition_size: u16,
    /// The actual buffers in the queue.
    /// Indexed by: Index => Value
    buffers: HashMap<u32, T>,
}

impl BufferQueue<Vec<u8>> {
    /// Creates a new buffer queue.
    pub fn new(size: u32, partition_size: u16) -> Self {
        BufferQueue {
            size,
            partition_size,
            buffers: HashMap::new(),
        }
    }

    /// Sets the partition size with the overhead of raknet taken into account.
    /// This usually isn't needed at this stage but **Can** be useful.
    // pub fn set_part_size_with_rak(&mut self, partition_size: u16) -> Result<(), QueueError> {
    //     if partition_size < 27 {
    //         // wtf this should NEVER happen but this is not good
    //         return Err(QueueError::Partition);
    //     }
    //     self.partition_size = partition_size - 27;
    //     Ok(())
    // }

    /// Adds a packet to the buffer queue.
    pub fn push(&mut self, packet: Vec<u8>) -> Result<(), QueueError> {
        if self.buffers.len() >= self.size as usize {
            return Err(QueueError::Full);
        }
        let index = self.buffers.len() as u32;
        self.buffers.insert(index, packet);
        Ok(())
    }

    /// Repartitions the buffers in the queue.
    /// This can be used if the MtuSize of the connection has changed.
    /// This should be called before sending the buffers to verify that the packets are not too large.
    /// It is important to note, this method does not preserve the order of the packets.
    pub fn partition(&mut self) {
        let mut new_buffers: HashMap<u32, Vec<u8>> = HashMap::new();
        let mut index = 0;
        for (_, buffer) in self.buffers.iter() {
            // check the buffer length
            let mut length = buffer.len();
            let mut current: Vec<u8> = buffer.clone();

            while length > self.partition_size.into() {
                // the length of this buffer is too large, split the buffer and add it into the new buffers
                let children = current.split_at(self.partition_size as usize);
                new_buffers.insert(index, children.0.to_vec());

                index += 1;

                if children.1.len() < self.partition_size as usize {
                    // the buffer is smaller than the partition size, we are done
                    new_buffers.insert(index, children.1.to_vec());
                    break;
                } else {
                    // the buffer is larger than the partition size, continue
                    current = children.1.to_vec();
                    length = current.len();
                }
            }

            index += 1;
        }
    }
}
