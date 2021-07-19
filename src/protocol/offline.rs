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
     IncompatibleProtocolVersion,
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
               0x19 => OfflinePackets::IncompatibleProtocolVersion,
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
               OfflinePackets::IncompatibleProtocolVersion => 0x19,
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
               OfflinePackets::IncompatibleProtocolVersion => write!(f, "{}", self.to_byte()),
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
               timestamp: stream.read_long(),
               magic: stream.read_magic(),
               client_id: stream.read_long(),
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
          stream.write_long(self.timestamp.try_into().unwrap());
          stream.write_long(self.server_id);
          stream.write_magic();
          stream.write_string(self.motd.parse());
          stream
     }
}

/// A connection request recv the client.
pub struct OpenConnectRequest {
     magic: Vec<u8>,
     protocol: u8,
     mtu_size: i16,
}

impl IServerBound<OpenConnectRequest> for OpenConnectRequest {
     fn recv(mut s: BinaryStream) -> OpenConnectRequest {
          let magic = s.read_magic();
          let p = s.read_byte();
          let mtu = s.get_length() + 1 + 28;
          OpenConnectRequest {
               magic,
               protocol: p,
               mtu_size: mtu as i16,
          }
     }
}

/// Open Connection Reply
/// Sent to the client when the server accepts a client.
pub struct OpenConnectReply {
     server_id: i64,
     security: bool,
     mtu_size: i16,
}

impl IClientBound<OpenConnectReply> for OpenConnectReply {
     fn to(&self) -> BinaryStream {
          let mut stream = BinaryStream::new();
          stream.write_byte(OfflinePackets::OpenConnectReply.to_byte());
          stream.write_magic();
          stream.write_long(self.server_id);
          stream.write_bool(self.security);
          stream.write_short(self.mtu_size);
          stream
     }
}

/// Session info, also known as Open Connect Request 2
pub struct SessionInfoRequest {
     magic: Vec<u8>,
     address: SocketAddr,
     mtu_size: i16,
     client_id: i64,
}

impl IServerBound<SessionInfoRequest> for SessionInfoRequest {
     fn recv(mut stream: BinaryStream) -> SessionInfoRequest {
          Self {
               magic: stream.read_magic(),
               address: stream.read_address(),
               mtu_size: stream.read_short(),
               client_id: stream.read_long(),
          }
     }
}

/// Session Info Reply, also known as Open Connect Reply 2
pub struct SessionInfoReply {
     server_id: i64,
     client_address: SocketAddr,
     mtu_size: i16,
     security: bool,
}

impl IClientBound<SessionInfoReply> for SessionInfoReply {
     fn to(&self) -> BinaryStream {
          let mut stream: BinaryStream = BinaryStream::new();
          stream.write_byte(OfflinePackets::SessionInfoReply.to_byte());
          stream.write_magic();
          stream.write_long(self.server_id);
          stream.write_address(self.client_address);
          stream.write_short(self.mtu_size);
          stream.write_bool(self.security);
          stream
     }
}

pub struct IncompatibleProtocolVersion {
     protocol: u8,
     server_id: i64,
}

impl IClientBound<IncompatibleProtocolVersion> for IncompatibleProtocolVersion {
     fn to(&self) -> BinaryStream {
          let mut stream: BinaryStream = BinaryStream::new();
          stream.write_byte(OfflinePackets::IncompatibleProtocolVersion.to_byte());
          stream.write_byte(self.protocol);
          stream.write_magic();
          stream.write_long(self.server_id);
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
                    let incompatible = IncompatibleProtocolVersion {
                         protocol: request.protocol,
                         server_id: SERVER_ID
                    };

                    return incompatible.to();
               }

               let reply = OpenConnectReply {
                    server_id: SERVER_ID,
                    security: USE_SECURITY,
                    mtu_size: request.mtu_size,
               };

               reply.to()
          },
          OfflinePackets::SessionInfoRequest => {
               let request = SessionInfoRequest::recv(stream.clone());
               let reply = SessionInfoReply {
                    server_id: SERVER_ID,
                    client_address: connection.address.clone(),
                    mtu_size: request.mtu_size,
                    security: USE_SECURITY,
               };
               reply.to()
          }
          _ => BinaryStream::new(), //TODO: Throw an UnknownPacket here rather than sending an empty binary stream
     }
}
