/// Minecraft has specific protocol for the `UnconnectedPong` packet.
/// This data is attached to the Unconnect Pong packet and is used to
/// display information about the server.
pub mod motd;

use crate::packet_id;

use binary_utils::*;

use self::motd::Motd;

use super::{packet::PacketId, Magic};

#[derive(Debug, Clone, BinaryStream)]
pub struct UnconnectedPong {
    pub timestamp: u64,
    pub server_id: u64,
    pub magic: Magic,
    pub motd: Motd,
}
packet_id!(UnconnectedPong, 0x1c);
