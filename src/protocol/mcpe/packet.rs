use binary_utils::*;

use crate::protocol::PacketId;
use crate::{packet_id, protocol::util::Magic};

use super::motd::Motd;

#[derive(Debug, Clone, BinaryStream)]
pub struct UnconnectedPong {
    pub timestamp: u64,
    pub server_id: u64,
    pub magic: Magic,
    pub motd: Motd,
}
packet_id!(UnconnectedPong, 0x1c);
