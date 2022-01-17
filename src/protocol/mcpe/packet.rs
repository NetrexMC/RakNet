use binary_utils::*;

use crate::{protocol::util::Magic, packet_id};
use crate::protocol::PacketId;

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