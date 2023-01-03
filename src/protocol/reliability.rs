#[derive(Clone, Debug, Copy)]
#[repr(u8)]
pub enum Reliability {
    /// Unreliable (with no ack)
    Unreliable = 0,
    /// Unreliable with a sequence
    UnreliableSeq,
    /// Reliable
    Reliable,
    ReliableOrd,
    /// Reliably sequenced **AND** ordered
    ReliableSeq,
    /// never used over the wire
    UnreliableAck,
    /// never used over the wire
    ReliableAck,
    /// never used over the wire
    ReliableOrdAck,
}

impl Reliability {
    pub fn from_flags(flags: u8) -> Self {
        match (flags & 224) >> 5 {
            0 => Reliability::Unreliable,
            1 => Reliability::UnreliableSeq,
            2 => Reliability::Reliable,
            3 => Reliability::ReliableOrd,
            4 => Reliability::ReliableSeq,
            5 => Reliability::UnreliableAck,
            6 => Reliability::ReliableAck,
            7 => Reliability::ReliableOrdAck,
            // we shouldn't error, but we'll just return unreliable
            _ => Reliability::Unreliable,
        }
    }

    pub fn to_flags(&self) -> u8 {
        match self {
            Reliability::Unreliable => 0 << 5,
            Reliability::UnreliableSeq => 1 << 5,
            Reliability::Reliable => 2 << 5,
            Reliability::ReliableOrd => 3 << 5,
            Reliability::ReliableSeq => 4 << 5,
            Reliability::UnreliableAck => 5 << 5,
            Reliability::ReliableAck => 6 << 5,
            Reliability::ReliableOrdAck => 7 << 5,
        }
    }

    /// Whether or not the packet is ordered.
    pub fn is_ordered(&self) -> bool {
        match self {
            Self::UnreliableSeq | Self::ReliableOrd | Self::ReliableOrdAck => true,
            _ => false,
        }
    }

    /// Whether or not the packet is reliable.
    pub fn is_reliable(&self) -> bool {
        match self {
            Self::Reliable | Self::ReliableOrd | Self::ReliableSeq | Self::ReliableOrdAck => true,
            _ => false,
        }
    }

    /// Whether or not the packet is unreliable.
    pub fn is_unreliable(&self) -> bool {
        match self {
            Self::Unreliable | Self::UnreliableSeq | Self::UnreliableAck => true,
            _ => false,
        }
    }

    /// Whether or not the packet is sequenced.
    pub fn is_sequenced(&self) -> bool {
        match self {
            Self::UnreliableSeq | Self::ReliableSeq => true,
            _ => false,
        }
    }

    pub fn is_sequenced_or_ordered(&self) -> bool {
        match self {
            Self::UnreliableSeq | Self::ReliableSeq | Self::ReliableOrd | Self::ReliableOrdAck => {
                true
            }
            _ => false,
        }
    }

    /// Whether or not the packet has an acknowledgement.
    pub fn is_ack(&self) -> bool {
        match self {
            Self::UnreliableAck | Self::ReliableAck | Self::ReliableOrdAck => true,
            _ => false,
        }
    }
}
