/// Minecraft has specific protocol for the `UnconnectedPong` packet.
/// This data is attached to the Unconnect Pong packet and is used to
/// display information about the server.
pub mod motd;

use binary_util::BinaryIo;

use self::motd::Motd;

use super::Magic;

#[derive(Debug, Clone, BinaryIo)]
pub struct UnconnectedPong {
    pub timestamp: u64,
    pub server_id: u64,
    pub magic: Magic,
    pub motd: Motd,
}
