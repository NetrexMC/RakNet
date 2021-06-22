#![allow(dead_code)]
use super::{ IClientBound, IServerBound };
use crate::{ IPacketStreamWrite, IPacketStreamRead };
use binary_utils::{ stream::*, IBufferRead, IBufferWrite };
// use std::net::{ IpAddr, SocketAddr };

pub enum OfflinePackets {
     UnconnectedPing = 0x01,
     OpenConnectRequest = 0x05,
     OpenConnectReply = 0x06,
     SessionInfo = 0x07,
     SessionInfoReply = 0x08,
     UnconnectedPong = 0x1c,
}

/// Open Connection Reply
/// Sent to the client when the server accepts a client.
pub struct OpenConnectReply {
     server_id: i64,
     security: bool,
     mtu: u16,
}

impl IClientBound<OpenConnectReply> for OpenConnectReply {
     fn to(packet: OpenConnectReply) -> BinaryStream {
          let mut stream = BinaryStream::new();
          stream.write_byte(OfflinePackets::OpenConnectReply as u16);
          stream.write_magic();
          stream.write_long(packet.server_id);
          stream.write_bool(packet.security);
          stream.write_short(packet.mtu);
          stream
     }
}

/// A connection request from the client.
pub struct OpenConnectRequest {
     protocol: u16,
     mtu_size: usize
}

impl IServerBound<OpenConnectRequest> for OpenConnectRequest {
     fn from(mut s: BinaryStream) -> OpenConnectRequest {
          let p = s.read_byte();
          let mtu = s.get_length() + 1 + 28;
          OpenConnectRequest {
               protocol: p,
               mtu_size: mtu
          }
     }
}

/// Session info, also known as Open Connect Request 2
pub struct SessionInfo {
     magic: Vec<u8>,
     // address: SocketAddr,
     mtu: usize,
     client_id: i64
}

impl IServerBound<SessionInfo> for SessionInfo {
     fn from(mut stream: BinaryStream) -> SessionInfo {
          Self {
               magic: stream.read_magic(),
               // address: SocketAddr::from((0,0,0,0)),
               mtu: stream.read_short() as usize,
               client_id: stream.read_long()
          }
     }
}