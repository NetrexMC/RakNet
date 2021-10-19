#![allow(dead_code)]

use crate::conn::{Connection, ConnectionState};
use crate::{IPacketStreamRead, IPacketStreamWrite, Magic, RakNetVersion, USE_SECURITY};
use crate::{Motd, SERVER_ID};
use crate::RakNetEvent;
use binary_utils::*;
use byteorder::{ReadBytesExt, WriteBytesExt};
use std::convert::TryInto;
use std::fmt::{Formatter, Result as FResult};
use std::net::SocketAddr;
use std::io::Cursor;

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
#[derive(Debug, BinaryStream)]
pub struct UnconnectedPing {
     timestamp: i64,
     magic: Magic,
     client_id: i64,
}

/// Unconnected Pong
#[derive(Debug, BinaryStream)]
pub struct UnconnectedPong {
     id: u8,
     timestamp: i64,
     server_id: i64,
     magic: Magic,
     motd: String,
}

/// A connection request recv the client.
#[derive(Debug, BinaryStream)]
pub struct OpenConnectRequest {
     magic: Magic,
     protocol: u8,
     mtu_size: i16,
}

// Mtu size may be needed here.
// impl IServerBound<OpenConnectRequest> for OpenConnectRequest {
//      fn recv(mut s: Stream) -> OpenConnectRequest {
//           let magic = s.read_magic();
//           let p = s.read_byte();
//           let mtu = s.get_length() + 1 + 28;
//           Self {
//                magic,
//                protocol: p,
//                mtu_size: mtu as i16,
//           }
//      }
// }

/// Open Connection Reply
/// Sent to the client when the server accepts a client.
#[derive(Debug, BinaryStream)]
pub struct OpenConnectReply {
     id: u8,
     magic: Magic,
     server_id: i64,
     security: bool,
     mtu_size: i16,
}
/// Session info, also known as Open Connect Request 2
#[derive(Debug, BinaryStream)]
pub struct SessionInfoRequest {
     magic: Magic,
     address: SocketAddr,
     mtu_size: i16,
     client_id: i64,
}

/// Session Info Reply, also known as Open Connect Reply 2
#[derive(Debug, BinaryStream)]
pub struct SessionInfoReply {
     id: u8,
     magic: Magic,
     server_id: i64,
     client_address: SocketAddr,
     mtu_size: i16,
     security: bool,
}

#[derive(Debug, BinaryStream)]
pub struct IncompatibleProtocolVersion {
     id: u8,
     protocol: u8,
     magic: Magic,
     server_id: i64,
}

pub fn handle_offline(
     connection: &mut Connection,
     pk: OfflinePackets,
     stream: &mut &Vec<u8>,
) -> Vec<u8> {
     match pk {
          OfflinePackets::UnconnectedPing => {
               let pong = UnconnectedPong {
                    id: OfflinePackets::UnconnectedPong.to_byte(),
                    server_id: SERVER_ID,
                    timestamp: connection.time.elapsed().unwrap().as_millis() as i64,
                    magic: Magic::new(),
                    motd: connection.get_motd().encode(),
               };
               pong.parse()
          }
          OfflinePackets::OpenConnectRequest => {
               let request = OpenConnectRequest::compose(&stream[..], &mut 1);

               if request.protocol != RakNetVersion::MinecraftRecent.to_u8() {
                    let incompatible = IncompatibleProtocolVersion {
                         id: OfflinePackets::IncompatibleProtocolVersion.to_byte(),
                         protocol: request.protocol,
                         magic: Magic::new(),
                         server_id: SERVER_ID,
                    };

                    return incompatible.parse();
               }

               let reply = OpenConnectReply {
                    id: OfflinePackets::OpenConnectReply.to_byte(),
                    server_id: SERVER_ID,
                    security: USE_SECURITY,
                    magic: Magic::new(),
                    mtu_size: request.mtu_size,
               };

               reply.parse()
          }
          OfflinePackets::SessionInfoRequest => {
               let request = SessionInfoRequest::compose(&stream[..], &mut 1);
               let reply = SessionInfoReply {
                    id: OfflinePackets::SessionInfoReply.to_byte(),
                    server_id: SERVER_ID,
                    client_address: connection.address.clone(),
                    magic: Magic::new(),
                    mtu_size: request.mtu_size,
                    security: USE_SECURITY,
               };

               connection.mtu_size = request.mtu_size as u16;
               connection.state = ConnectionState::Connecting;
               reply.parse()
          }
          _ => Vec::new(), //TODO: Throw an UnknownPacket here rather than sending an empty binary stream
     }
}
