//! Reliability types for packets.
//! Each packet sent on the RakNet protocol has a reliability type, which determines how the packet should be handled.
//!
//! ## Unreliable
//! Unreliable packets are sent without any guarantee that they will be received by the other peer.
//! In other words, the packet is sent and forgotten about, with no expectation of a response.
//! This is the fastest reliability type, as it does not require any acks to be sent
//! back to the sender. Generally used for packets that are not important, such as
//! [`UnconnectedPing`] and [`UnconnectedPong`].
//!
//! ## Sequenced
//! Sequenced packets are sent with a sequence index in each frame (datagram) that is sent.
//! This is a number that is incremented after each packet. This allows us to implement a sliding window
//! ([`ReliableWindow`]), which
//! will discard any packets that are way too old or way too new.
//!
//! ## Ordered
//! Ordered packets are sent with an order index in each frame, as well as an order channel.
//! RakNet will use this information to order the packets in a specific way.
//!
//! The `order_channel` is used to determine which channel the packet should be sent on, in other words,
//! this is a unique channel chosen by the sender to send the packet on, and the receiver will use this
//! channel to re-order the packets in the correct order when they are sent.
//!
//! The `order_index` is used to determine the position of the packet in the order channel, this is
//! incremented after each packet is sent, similar to the sequence index, without the sliding window.
//!
//! ## Reliable
//! Reliable packets are sent with an ack system, meaning that the receiver will send an ack back to the
//! sender, which the sender expects to recieve.
//!
//! If the sender does not receive the ack, it will resend the packet until it receives the ack, or until
//! the connection is closed.
//!
//! [`UnconnectedPing`]: crate::protocol::packet::offline::UnconnectedPing
//! [`UnconnectedPong`]: crate::protocol::packet::offline::UnconnectedPong
//! [`ReliableWindow`]: crate::connection::controller::window::ReliableWindow
//!
/// The [RakNet Reliabilty] of a packet.
///
/// This is a bit flag encoded within each [`Frame`] that determines how the packet should be handled.
/// As of writing the following reliability types are supported:
/// - [`Unreliable`]
/// - [`UnreliableSeq`]
/// - [`Reliable`]
/// - [`ReliableOrd`]
/// - [`ReliableSeq`]
///
/// [RakNet Reliabilty]: https://github.com/facebookarchive/RakNet/blob/1a169895a900c9fc4841c556e16514182b75faf8/Source/PacketPriority.h#L46-L85
/// [`Frame`]: crate::protocol::frame::Frame
/// [`Unreliable`]: crate::protocol::reliability::Reliability::Unreliable
/// [`UnreliableSeq`]: crate::protocol::reliability::Reliability::UnreliableSeq
/// [`Reliable`]: crate::protocol::reliability::Reliability::Reliable
/// [`ReliableOrd`]: crate::protocol::reliability::Reliability::ReliableOrd
/// [`ReliableSeq`]: crate::protocol::reliability::Reliability::ReliableSeq
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
    /// Creates a new [`Reliability`] from the given flags.
    /// This is used internally to decode the reliability from the given
    /// bit flags.
    ///
    /// [`Reliability`]: crate::protocol::reliability::Reliability
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

    /// Converts the [`Reliability`] into a bit flag.
    /// This is useful for encoding the reliability into a packet.
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

    /// This method checks whether the reliability is ordered, meaning that the packets
    /// are either:
    /// - [`ReliableOrd`]
    /// - [`ReliableOrdAck`]
    /// - [`UnreliableSeq`]
    ///
    /// [`ReliableOrd`]: crate::protocol::reliability::Reliability::ReliableOrd
    /// [`ReliableOrdAck`]: crate::protocol::reliability::Reliability::ReliableOrdAck
    /// [`UnreliableSeq`]: crate::protocol::reliability::Reliability::UnreliableSeq
    pub fn is_ordered(&self) -> bool {
        match self {
            Self::UnreliableSeq | Self::ReliableOrd | Self::ReliableOrdAck => true,
            _ => false,
        }
    }

    /// Verifies whether or not the reliabilty is reliable, meaning that the packets
    /// are either:
    /// - [`Reliable`]
    /// - [`ReliableOrd`]
    /// - [`ReliableSeq`]
    /// - [`ReliableAck`]
    ///
    /// Other reliabilities are not reliable, and will return `false`.
    ///
    /// [`Reliable`]: crate::protocol::reliability::Reliability::Reliable
    /// [`ReliableOrd`]: crate::protocol::reliability::Reliability::ReliableOrd
    /// [`ReliableSeq`]: crate::protocol::reliability::Reliability::ReliableSeq
    /// [`ReliableAck`]: crate::protocol::reliability::Reliability::ReliableAck
    pub fn is_reliable(&self) -> bool {
        match self {
            Self::Reliable | Self::ReliableOrd | Self::ReliableSeq | Self::ReliableOrdAck => true,
            _ => false,
        }
    }

    /// Verifies whether or not the reliabilty is unreliable, meaning that the packets
    /// are either:
    /// - [`Unreliable`]
    /// - [`UnreliableSeq`]
    /// - [`UnreliableAck`]
    ///
    /// Other reliabilities are not unreliable, and will return `false`.
    ///
    /// [`Unreliable`]: crate::protocol::reliability::Reliability::Unreliable
    /// [`UnreliableSeq`]: crate::protocol::reliability::Reliability::UnreliableSeq
    /// [`UnreliableAck`]: crate::protocol::reliability::Reliability::UnreliableAck
    pub fn is_unreliable(&self) -> bool {
        match self {
            Self::Unreliable | Self::UnreliableSeq | Self::UnreliableAck => true,
            _ => false,
        }
    }

    /// Verifies whether or not the reliabilty is sequenced, meaning that the packets
    /// are either:
    /// - [`UnreliableSeq`]
    /// - [`ReliableSeq`]
    ///
    /// Other reliabilities are not sequenced, and will return `false`.
    ///
    /// ## What is a sequenced packet?
    /// A sequenced packet is a packet with an index that is incremented after each packet.
    /// RakNet uses this internally to discard packets that are sent out of sequence, accepting
    /// only the latest packet.
    ///
    /// [`UnreliableSeq`]: crate::protocol::reliability::Reliability::UnreliableSeq
    /// [`ReliableSeq`]: crate::protocol::reliability::Reliability::ReliableSeq
    pub fn is_sequenced(&self) -> bool {
        match self {
            Self::UnreliableSeq | Self::ReliableSeq => true,
            _ => false,
        }
    }

    /// Verifies whether or not the reliabilty is sequenced or ordered. This function
    /// is a combination of [`Reliability::is_sequenced`] and [`Reliability::is_ordered`],
    /// combined into one signature.
    ///
    /// [`Reliability::is_sequenced`]: crate::protocol::reliability::Reliability::is_sequenced
    /// [`Reliability::is_ordered`]: crate::protocol::reliability::Reliability::is_ordered
    pub fn is_sequenced_or_ordered(&self) -> bool {
        match self {
            Self::UnreliableSeq | Self::ReliableSeq | Self::ReliableOrd | Self::ReliableOrdAck => {
                true
            }
            _ => false,
        }
    }

    /// Verifies that the reliability is an ack ([`Ack`]).
    /// This is primarily used internally to determine whether or not to send an ack.
    ///
    /// [`Ack`]: crate::protocol::ack::Ack
    pub fn is_ack(&self) -> bool {
        match self {
            Self::UnreliableAck | Self::ReliableAck | Self::ReliableOrdAck => true,
            _ => false,
        }
    }
}
