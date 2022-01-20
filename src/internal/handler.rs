use std::collections::{HashSet, HashMap};

use super::{queue::BufferQueue, ack::Ack};

/// The handler for Ack, Nack and Frame packets.
/// This does not handle the actual sending of packets,
pub struct RakSessionHandler {
    /// The next Non-Acked packets that should be sent.
    /// These are packets we expect back from the client, but have not gotten.
    pub nack: HashSet<u32>,
    /// The Acked packets that have been sent, waiting for ack back. (only if reliable)
    /// This is used to determine if the packet has been received.
    /// We're also storing the count of how many times we've sent a packet.
    /// If it's been sent more than once, we'll consider it lost and remove it.
    pub ack: BufferQueue<Vec<u8>>,
    /// A queue to send back to the client to acknowledge we've recieved these packets.
    pub ack_counts: HashSet<u32>,
    /// The ordered channels that have been recieved and are waiting for completion.
    /// Ordered channels will be reorded once all the packets have been received.
    pub ordered_channels: HashMap<u8, HashMap<u16, Vec<u8>>>,
    /// The fragmented frames that are waiting for reassembly.
    pub fragmented_frames: HashMap<u32, HashMap<u16, Vec<u8>>>,
}