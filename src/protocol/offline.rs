#![allow(dead_code)]

use super::{IClientBound, IServerBound};
use crate::conn::Connection;
use crate::{IPacketStreamRead, IPacketStreamWrite, MTU_SIZE, USE_SECURITY};
use crate::{Motd, SERVER_ID};
use binary_utils::{stream::*, IBufferRead, IBufferWrite};
use std::convert::TryInto;
use std::fmt::{Formatter, Result as FResult};
use std::net::SocketAddr;
// use crate::offline::OfflinePackets::UnknownPacket;

pub enum OfflinePackets {
     UnconnectedPing,
     OpenConnectRequest,
     OpenConnectReply,
     SessionInfoRequest,
     SessionInfoReply,
     UnconnectedPong,
     UnknownPacket(u8),
}

impl OfflinePackets {
     pub fn recv(byte: u8) -> Self {
          match byte {
               0x01 => OfflinePackets::UnconnectedPing,
               0x05 => OfflinePackets::OpenConnectRequest,
               0x06 => OfflinePackets::OpenConnectReply,
               0x07 => OfflinePackets::SessionInfoRequest,
               0x08 => OfflinePackets::SessionInfoReply,
               0x1c => OfflinePackets::UnconnectedPong,
               _ => OfflinePackets::UnknownPacket(byte),
          }
     }

     pub fn to_byte(&self) -> u8 {
          match *self {
               OfflinePackets::UnconnectedPing => 0x01,
               OfflinePackets::OpenConnectRequest => 0x05,
               OfflinePackets::OpenConnectReply => 0x06,
               OfflinePackets::SessionInfoRequest => 0x07,
               OfflinePackets::SessionInfoReply => 0x08,
               OfflinePackets::UnconnectedPong => 0x1c,
               OfflinePackets::UnknownPacket(byte) => byte,
          }
     }
}

impl std::fmt::Display for OfflinePackets {
     fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
          match *self {
               OfflinePackets::UnconnectedPing => write!(f, "{}", self.to_byte()),
               OfflinePackets::OpenConnectRequest => write!(f, "{}", self.to_byte()),
               OfflinePackets::OpenConnectReply => write!(f, "{}", self.to_byte()),
               OfflinePackets::SessionInfoRequest => write!(f, "{}", self.to_byte()),
               OfflinePackets::SessionInfoReply => write!(f, "{}", self.to_byte()),
               OfflinePackets::UnconnectedPong => write!(f, "{}", self.to_byte()),
               OfflinePackets::UnknownPacket(byte) => write!(f, "{}", byte),
          }
     }
}

/// Unconnected Ping
pub struct UnconnectedPing {
     timestamp: i64,
     magic: Vec<u8>,
     client_id: i64,
}

impl IServerBound<UnconnectedPing> for UnconnectedPing {
     fn recv(mut stream: BinaryStream) -> UnconnectedPing {
          Self {
               timestamp: stream.read_signed_long(),
               magic: stream.read_magic(),
               client_id: stream.read_signed_long(),
          }
     }
}

/// Unconnected Pong
pub struct UnconnectedPong {
     timestamp: i64,
     server_id: i64,
     motd: Motd,
}

impl IClientBound<UnconnectedPong> for UnconnectedPong {
     fn to(&self) -> BinaryStream {
          let mut stream = BinaryStream::new();
          stream.write_byte(OfflinePackets::UnconnectedPong.to_byte());
          stream.write_signed_long(self.timestamp.try_into().unwrap());
          stream.write_signed_long(self.server_id);
          stream.write_magic();
          stream.write_string(self.motd.parse());
          stream
     }
}

/// A connection request recv the client.
pub struct OpenConnectRequest {
     protocol: u16,
     mtu_size: usize,
}

impl IServerBound<OpenConnectRequest> for OpenConnectRequest {
     fn recv(mut s: BinaryStream) -> OpenConnectRequest {
          let p = s.read_short();
          let mtu = s.get_length() + 1 + 28;
          OpenConnectRequest {
               protocol: p,
               mtu_size: mtu,
          }
     }
}

/// Open Connection Reply
/// Sent to the client when the server accepts a client.
pub struct OpenConnectReply {
     server_id: i64,
     security: bool,
     mtu: i16,
}

impl IClientBound<OpenConnectReply> for OpenConnectReply {
     fn to(&self) -> BinaryStream {
          let mut stream = BinaryStream::new();
          stream.write_byte(OfflinePackets::OpenConnectReply.to_byte());
          stream.write_magic();
          stream.write_signed_long(self.server_id);
          stream.write_bool(self.security);
          stream.write_signed_short(self.mtu);
          stream
     }
}

/// Session info, also known as Open Connect Request 2
pub struct SessionInfoRequest {
     magic: Vec<u8>,
     address: SocketAddr,
     mtu: u16,
     client_id: i64,
}

impl IServerBound<SessionInfoRequest> for SessionInfoRequest {
     fn recv(mut stream: BinaryStream) -> SessionInfoRequest {
          Self {
               magic: stream.read_magic(),
               address: stream.read_address(),
               mtu: stream.read_short(),
               client_id: stream.read_signed_long(),
          }
     }
}

/// Session Info Reply, also known as Open Connect Reply 2
pub struct SessionInfoReply {
     server_id: i64,
     client_address: SocketAddr,
     mtu: u16,
     security: bool,
}

impl IClientBound<SessionInfoReply> for SessionInfoReply {
     fn to(&self) -> BinaryStream {
          let mut stream: BinaryStream = BinaryStream::new();
          stream.write_byte(OfflinePackets::SessionInfoReply.to_byte());
          stream.write_magic();
          stream.write_signed_long(self.server_id);
          stream.write_address(self.client_address);
          stream.write_short(self.mtu);
          stream.write_bool(self.security);
          stream
     }
}

pub fn handle_offline(
     connection: &mut Connection,
     pk: OfflinePackets,
     stream: &mut BinaryStream,
) -> BinaryStream {
     match pk {
          OfflinePackets::UnconnectedPing => {
               let pong = UnconnectedPong {
                    server_id: SERVER_ID,
                    timestamp: connection.time.elapsed().unwrap().as_millis() as i64,
                    motd: connection.motd.clone(),
               };

               pong.to()
          }
          OfflinePackets::OpenConnectRequest => {
               let request = OpenConnectRequest::recv(stream.clone());

               if request.protocol != 10 {
                    // disconnect
               }

               let reply = OpenConnectReply {
                    server_id: SERVER_ID,
                    security: USE_SECURITY,
                    mtu: MTU_SIZE,
               };

               reply.to()
          },
          OfflinePackets::SessionInfoRequest => {
               let request = SessionInfoRequest::recv(stream.clone());
               let reply = SessionInfoReply {
                    server_id: SERVER_ID,
                    client_address: connection.address.clone(),
                    mtu: request.mtu,
                    security: USE_SECURITY,
               };
               reply.to()
          }
          _ => BinaryStream::new(), //TODO: Throw an UnknownPacket here rather than sending an empty binary stream
     }
}
