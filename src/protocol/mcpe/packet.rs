use binary_utils::*;

use crate::protocol::PacketId;
use crate::{packet_id, protocol::util::Magic};

use super::motd::Motd;

#[derive(Debug, Clone, BinaryStream)]
pub struct UnconnectedPong {
    id: u8,
    timestamp: u64,
    server_id: u64,
    magic: Magic,
    motd: Motd,
}
packet_id!(UnconnectedPong, 0x1c);
