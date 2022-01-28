/// Minecrafts specific protocol for the `UnconnectedPong` packet.
/// This data is attached to the Unconnect Pong packet and is used to
/// display information about the server.
///
/// EG:
/// ```markdown
/// Netrex Server v1.18.0 - 1/1 players online
/// ```
pub mod motd;

/// Packet specific data for Minecraft, for now it's just the `UnconnectedPong` packet.
pub mod packet;
