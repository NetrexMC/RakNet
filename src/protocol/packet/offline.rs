//!  Offline packets are packets that are sent before a connection is established.
//! In rak-rs, these packets consist of:
//! - [`UnconnectedPing`]
//! - [`UnconnectedPong`]
//! - [`OpenConnectRequest`]
//! - [`OpenConnectReply`]
//! - [`SessionInfoRequest`]
//! - [`SessionInfoReply`]
//! - [`IncompatibleProtocolVersion`]
//!
//! During this stage, the client and server are exchanging information about each other, such as
//! the server id, the client id, the mtu size, etc, to prepare for the connection handshake.
use std::net::SocketAddr;

use super::RakPacket;
#[cfg(feature = "mcpe")]
pub use crate::protocol::mcpe::UnconnectedPong;
use crate::protocol::Magic;
use crate::protocol::RAKNET_HEADER_FRAME_OVERHEAD;
use crate::register_packets;

use binary_util::interfaces::{Reader, Writer};
use binary_util::io::{ByteReader, ByteWriter};
use binary_util::BinaryIo;

/// This is an enum of all offline packets.
///
/// You can use this to read and write offline packets,
/// with the `binary_util` traits `Reader` and `Writer`.
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

/// Send to the other peer expecting a [`UnconnectedPong`] packet,
/// this is used to determine the latency between the client and the server,
/// and to determine if the server is online.
///
/// If the peer does not respond with a [`UnconnectedPong`] packet, the iniatior should
/// expect that the server is offline.
#[derive(Debug, Clone, BinaryIo)]
pub struct UnconnectedPing {
    pub timestamp: u64,
    pub magic: Magic,
    pub client_id: i64,
}

/// Sent in response to a [`UnconnectedPing`] packet.
/// This is used to determine the latency between the client and the server, and to determine
/// that the peer is online.
///
/// <style>
/// .warning-2 {
///     background: rgba(255,240,76,0.34) !important;
///     padding: 0.75em;
///     border-left: 2px solid #fce811;
///     font-family: "Source Serif 4", NanumBarunGothic, serif;
///  }
///
/// .warning-2 code {
///     background: rgba(211,201,88,0.64) !important;
/// }
///
/// .notice-2 {
///     background: rgba(88, 211, 255, 0.34) !important;
///     padding: 0.75em;
///     border-left: 2px solid #4c96ff;
///     font-family: "Source Serif 4", NanumBarunGothic, serif;
/// }
///
/// .notice-2 code {
///     background: rgba(88, 211, 255, 0.64) !important;
/// }
/// </style>
/// <div class="notice-2">
///     <strong> Note: </strong>
///    <p>
///         If the client is a Minecraft: Bedrock Edition client, this packet is not sent
///         and the
///         <a
///             href="/rak-rs/latest/protocol/mcpe/struct.UnconnectedPong.html"
///             title="struct rak_rs::protocol::mcpe::UnconnectedPing">
///             UnconnectedPong
///         </a>
///         from the <code>mcpe</code> module is sent instead.
///   </p>
/// </div>
///
/// [`UnconnectedPong`]: crate::protocol::packet::offline::UnconnectedPong
#[cfg(not(feature = "mcpe"))]
#[derive(Debug, Clone, BinaryIo)]
pub struct UnconnectedPong {
    pub timestamp: u64,
    pub server_id: u64,
    pub magic: Magic,
}

/// This packet is the equivelant of the `OpenConnectRequest` packet in RakNet.
///
/// This packet is sent by the peer to a server to request a connection.
/// It contains information about the client, such as the protocol version, and the mtu size.
/// The peer should expect a [`OpenConnectReply`] packet in response to this packet, if the
/// server accepts the connection. Otherwise, the peer should expect a [`IncompatibleProtocolVersion`]
/// packet to be sent to indicate that the server does not support the protocol version.
///
/// <style>
/// .warning-2 {
///     background: rgba(255,240,76,0.34) !important;
///     padding: 0.75em;
///     border-left: 2px solid #fce811;
///     font-family: "Source Serif 4", NanumBarunGothic, serif;
///  }
///
/// .warning-2 code {
///     background: rgba(211,201,88,0.64) !important;
/// }
///
/// .notice-2 {
///     background: rgba(88, 211, 255, 0.34) !important;
///     padding: 0.75em;
///     border-left: 2px solid #4c96ff;
///     font-family: "Source Serif 4", NanumBarunGothic, serif;
/// }
///
/// .notice-2 code {
///     background: rgba(88, 211, 255, 0.64) !important;
/// }
/// </style>
/// <div class="notice-2">
///     <strong> Note: </strong>
///    <p>
///         Internally this packet is padded by the given
///         <code>mtu_size</code> in the packet. This is done by appending null bytes
///         to the current buffer of the packet which is calculated by adding the difference
///         between the <code>mtu_size</code> and the current length.
///   </p>
/// </div>
#[derive(Debug, Clone)]
pub struct OpenConnectRequest {
    pub protocol: u8,  // 9
    pub mtu_size: u16, // 500
}

impl Reader<OpenConnectRequest> for OpenConnectRequest {
    fn read(buf: &mut ByteReader) -> Result<OpenConnectRequest, std::io::Error> {
        let len = buf.as_slice().len();
        buf.read_type::<Magic>()?;
        Ok(OpenConnectRequest {
            protocol: buf.read_u8()?,
            mtu_size: (len + RAKNET_HEADER_FRAME_OVERHEAD as usize) as u16,
        })
    }
}

impl Writer for OpenConnectRequest {
    fn write(&self, buf: &mut ByteWriter) -> Result<(), std::io::Error> {
        buf.write_type::<Magic>(&Magic::new())?;
        buf.write_u8(self.protocol)?;
        // padding
        // remove 28 bytes from the mtu size
        let mtu_size = self.mtu_size - RAKNET_HEADER_FRAME_OVERHEAD as u16;
        for _ in 0..mtu_size {
            buf.write_u8(0)?;
        }
        Ok(())
    }
}

// Open Connection Reply
/// This packet is sent in response to a [`OpenConnectRequest`] packet, and confirms
/// the information sent by the peer in the [`OpenConnectRequest`] packet.
///
/// This packet is the equivalent of the `Open Connect Reply 1` within the original RakNet implementation.
///
/// If the server chooses to deny the connection, it should send a [`IncompatibleProtocolVersion`]
/// or ignore the packet.
#[derive(Debug, Clone, BinaryIo)]
pub struct OpenConnectReply {
    pub magic: Magic,
    pub server_id: u64,
    pub security: bool,
    pub mtu_size: u16,
}

/// This packet is sent after receiving a [`OpenConnectReply`] packet, and confirms
/// that the peer wishes to proceed with the connection. The information within this packet
/// is primarily used to get the external address of the peer.
///
/// This packet is the equivalent of the `Open Connect Request 2` within the original RakNet implementation.
#[derive(Debug, Clone, BinaryIo)]
pub struct SessionInfoRequest {
    pub magic: Magic,
    /// The socket address of the peer you are sending
    /// this packet to.
    pub address: SocketAddr,
    /// The mtu size of the peer you are sending this packet to.
    pub mtu_size: u16,
    /// Your internal client id.
    pub client_id: i64,
}

/// This packet is sent in response to a [`SessionInfoRequest`] packet, and confirms
/// all the information sent by the peer in the [`SessionInfoRequest`] packet. This packet
/// also specifies the external address of the peer, as well as whether or not
/// encryption at the RakNet level is enabled on the server.
///
/// This packet is the equivalent of the `Open Connect Reply 2` within the original RakNet implementation.
#[derive(Debug, Clone, BinaryIo)]
pub struct SessionInfoReply {
    pub magic: Magic,
    pub server_id: u64,
    pub client_address: SocketAddr,
    pub mtu_size: u16,
    pub security: bool,
}

/// This packet is sent by the server to indicate that the server does not support the
/// protocol version of the client.
#[derive(Debug, Clone, BinaryIo)]
pub struct IncompatibleProtocolVersion {
    pub protocol: u8,
    pub magic: Magic,
    pub server_id: u64,
}
