use binary_utils::*;
use binary_utils::u24;

/// Frames are a encapsulation of a packet or packets.
/// They are used to send packets to the connection in a reliable way.
pub struct FramePacket {
    /// The sequence of this frame.
    /// We'll use this to respond with Ack and Nack to.
    pub sequence: LE<u24>,

}