use std::net::SocketAddr;

use super::RakPacket;
#[cfg(feature = "mcpe")]
pub use crate::protocol::mcpe::UnconnectedPong;
use crate::protocol::Magic;
use crate::protocol::RAKNET_HEADER_OVERHEAD;
use crate::register_packets;

use binary_util::interfaces::{Reader, Writer};
use binary_util::io::{ByteReader, ByteWriter};
use binary_util::BinaryIo;

/// A enum that represents all offline packets.
#[derive(Clone, Debug, BinaryIo)]
#[repr(u8)]
pub enum OfflinePacket {
    UnconnectedPing(UnconnectedPing) = 0x01,
    UnconnectedPong(UnconnectedPong) = 0x1c,
    OpenConnectRequest(OpenConnectRequest) = 0x05,
    OpenConnectReply(OpenConnectReply) = 0x06,
    SessionInfoRequest(SessionInfoRequest) = 0x07,
    SessionInfoReply(SessionInfoReply) = 0x08,
    IncompatibleProtocolVersion(IncompatibleProtocolVersion) = 0x19,
}

register_packets! {
    Offline is OfflinePacket,
    UnconnectedPing,
    UnconnectedPong,
    OpenConnectRequest,
    OpenConnectReply,
    SessionInfoRequest,
    SessionInfoReply,
    IncompatibleProtocolVersion
}

/// Unconnected Ping
#[derive(Debug, Clone, BinaryIo)]
pub struct UnconnectedPing {
    pub timestamp: u64,
    pub magic: Magic,
    pub client_id: i64,
}

/// Unconnected Pong
#[cfg(not(feature = "mcpe"))]
#[derive(Debug, Clone, BinaryIo)]
pub struct UnconnectedPong {
    pub timestamp: u64,
    pub server_id: u64,
    pub magic: Magic,
}

/// This packet is the equivelant of the `OpenConnectRequest` packet in RakNet.
#[derive(Debug, Clone)]
pub struct OpenConnectRequest {
    pub protocol: u8,  // 9
    pub mtu_size: u16, // 500
}

impl Reader<OpenConnectRequest> for OpenConnectRequest {
    fn read(buf: &mut ByteReader) -> Result<OpenConnectRequest, std::io::Error> {
        let len = buf.as_slice().len();
        buf.read_struct::<Magic>()?;
        Ok(OpenConnectRequest {
            protocol: buf.read_u8()?,
            mtu_size: (len + 1 + 28) as u16,
        })
    }
}

impl Writer for OpenConnectRequest {
    fn write(&self, buf: &mut ByteWriter) -> Result<(), std::io::Error> {
        buf.write_type::<Magic>(&Magic::new())?;
        buf.write_u8(self.protocol)?;
        // padding
        // remove 28 bytes from the mtu size
        let mtu_size = self.mtu_size - buf.as_slice().len() as u16 - RAKNET_HEADER_OVERHEAD as u16;
        for _ in 0..mtu_size {
            buf.write_u8(0)?;
        }
        Ok(())
    }
}

// Open Connection Reply
/// Sent to the client when the server accepts a client.
/// This packet is the equivalent of the `Open Connect Reply 1` packet.
#[derive(Debug, Clone, BinaryIo)]
pub struct OpenConnectReply {
    pub magic: Magic,
    pub server_id: u64,
    pub security: bool,
    pub mtu_size: u16,
}

/// Session info, also known as Open Connect Request 2
#[derive(Debug, Clone, BinaryIo)]
pub struct SessionInfoRequest {
    pub magic: Magic,
    pub address: SocketAddr,
    pub mtu_size: u16,
    pub client_id: i64,
}

/// Session Info Reply, also known as Open Connect Reply 2
#[derive(Debug, Clone, BinaryIo)]
pub struct SessionInfoReply {
    pub magic: Magic,
    pub server_id: u64,
    pub client_address: SocketAddr,
    pub mtu_size: u16,
    pub security: bool,
}

#[derive(Debug, Clone, BinaryIo)]
pub struct IncompatibleProtocolVersion {
    pub protocol: u8,
    pub magic: Magic,
    pub server_id: u64,
}
