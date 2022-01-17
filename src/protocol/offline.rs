use std::io::Write;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

use binary_utils::*;
use binary_utils::error::BinaryError;
use byteorder::BigEndian;
use byteorder::WriteBytesExt;

use crate::{packet_id, register_packets};
use super::PacketId;
use super::Packet;
use super::Payload;
use super::util::Magic;

/// A enum that represents all offline packets.
#[derive(Clone, Debug)]
pub enum OfflinePacket {
    UnconnectedPing(UnconnectedPing),
    OpenConnectRequest(OpenConnectRequest),
    OpenConnectReply,
    SessionInfoRequest,
    SessionInfoReply,
    UnconnectedPong(UnconnectedPong),
    IncompatibleProtocolVersion
}

register_packets![
    Offline is OfflinePacket,
    UnconnectedPing,
    UnconnectedPong,
    OpenConnectRequest
];

/// Unconnected Ping
#[derive(Debug, Clone, BinaryStream)]
pub struct UnconnectedPing {
    timestamp: u64,
    magic: Magic,
    client_id: i64,
}
packet_id!(UnconnectedPing, 0x01);

/// Unconnected Pong
#[derive(Debug, Clone, BinaryStream)]
pub struct UnconnectedPong {
    id: u8,
    timestamp: u64,
    server_id: u64,
    magic: Magic,
    motd: String,
}
packet_id!(UnconnectedPong, 0x1c);

/// A connection request recv the client.
#[derive(Debug, Clone)]
pub struct OpenConnectRequest {
    magic: Magic,
    protocol: u8,
    mtu_size: u16,
}
packet_id!(OpenConnectRequest, 0x05);