use std::io::Write;
use std::net::SocketAddr;

use binary_utils::error::BinaryError;
use binary_utils::*;
use byteorder::WriteBytesExt;

#[cfg(feature = "mcpe")]
pub use crate::protocol::mcpe::UnconnectedPong;

use super::Packet;
use super::PacketId;
use super::Payload;
use crate::protocol::Magic;
use crate::{packet_id, register_packets};

/// A enum that represents all offline packets.
#[derive(Clone, Debug)]
pub enum OfflinePacket {
    UnconnectedPing(UnconnectedPing),
    OpenConnectRequest(OpenConnectRequest),
    OpenConnectReply(OpenConnectReply),
    SessionInfoRequest(SessionInfoRequest),
    SessionInfoReply(SessionInfoReply),
    #[cfg(feature = "mcpe")]
    UnconnectedPong(UnconnectedPong),
    #[cfg(not(feature = "mcpe"))]
    UnconnectedPong(UnconnectedPong),
    IncompatibleProtocolVersion(IncompatibleProtocolVersion),
}

register_packets![
    Offline is OfflinePacket,
    UnconnectedPing,
    UnconnectedPong,
    OpenConnectRequest,
    OpenConnectReply,
    SessionInfoRequest,
    SessionInfoReply,
    IncompatibleProtocolVersion
];

/// Unconnected Ping
#[derive(Debug, Clone, BinaryStream)]
pub struct UnconnectedPing {
    pub timestamp: u64,
    pub magic: Magic,
    pub client_id: i64,
}
packet_id!(UnconnectedPing, 0x01);

/// Unconnected Pong
#[cfg(not(feature = "mcpe"))]
#[derive(Debug, Clone, BinaryStream)]
pub struct UnconnectedPong {
    pub timestamp: u64,
    pub server_id: u64,
    pub magic: Magic,
}
#[cfg(not(feature = "mcpe"))]
packet_id!(UnconnectedPong, 0x1c);

/// This packet is the equivelant of the `OpenConnectRequest` packet in RakNet.
#[derive(Debug, Clone)]
pub struct OpenConnectRequest {
    pub magic: Magic,
    pub protocol: u8,  // 9
    pub mtu_size: u16, // 500
}
impl Streamable for OpenConnectRequest {
    fn compose(source: &[u8], position: &mut usize) -> Result<Self, BinaryError> {
        Ok(Self {
            magic: Magic::compose(source, position)?,
            protocol: u8::compose(source, position)?,
            mtu_size: (source.len() + 1 + 28) as u16,
        })
    }

    fn parse(&self) -> Result<Vec<u8>, BinaryError> {
        let mut stream = Vec::<u8>::new();
        stream
            .write(&self.magic.parse()?[..])
            .expect("Failed to parse open connect request");
        stream.write_u8(self.protocol)?;
        // padding
        for _ in 0..self.mtu_size {
            stream.write_u8(0)?;
        }
        Ok(stream)
    }
}
packet_id!(OpenConnectRequest, 0x05);

// Open Connection Reply
/// Sent to the client when the server accepts a client.
/// This packet is the equivalent of the `Open Connect Reply 1` packet.
#[derive(Debug, Clone, BinaryStream)]
pub struct OpenConnectReply {
    pub magic: Magic,
    pub server_id: u64,
    pub security: bool,
    pub mtu_size: u16,
}
packet_id!(OpenConnectReply, 0x06);

/// Session info, also known as Open Connect Request 2
#[derive(Debug, Clone, BinaryStream)]
pub struct SessionInfoRequest {
    pub magic: Magic,
    pub address: SocketAddr,
    pub mtu_size: u16,
    pub client_id: i64,
}
packet_id!(SessionInfoRequest, 0x07);

/// Session Info Reply, also known as Open Connect Reply 2
#[derive(Debug, Clone, BinaryStream)]
pub struct SessionInfoReply {
    pub magic: Magic,
    pub server_id: u64,
    pub client_address: SocketAddr,
    pub mtu_size: u16,
    pub security: bool,
}
packet_id!(SessionInfoReply, 0x08);

#[derive(Debug, Clone, BinaryStream)]
pub struct IncompatibleProtocolVersion {
    pub protocol: u8,
    pub magic: Magic,
    pub server_id: u64,
}
packet_id!(IncompatibleProtocolVersion, 0x19);
