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
            frozen: false
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
