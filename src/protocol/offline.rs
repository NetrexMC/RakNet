#![allow(dead_code)]

use super::{ IClientBound, IServerBound };
use crate::{ IPacketStreamWrite, IPacketStreamRead };
use crate::conn::{ Connection, ConnectionAPI };
use crate::util::*;
use crate::{ SERVER_ID, MAGIC, Motd };
use binary_utils::{ stream::*, IBufferRead, IBufferWrite };
use std::net::{ SocketAddr };

pub enum OfflinePackets {
     UnconnectedPing = 0x01,
     OpenConnectRequest = 0x05,
     OpenConnectReply = 0x06,
     SessionInfo = 0x07,
     SessionInfoReply = 0x08,
     UnconnectedPong = 0x1c,
     UnknownPacket = 0xAE
}

impl OfflinePackets {
     pub fn recv(byte: u16) -> Self {
          match byte {
               0x01 => OfflinePackets::UnconnectedPing,
               0x05 => OfflinePackets::OpenConnectRequest,
               0x06 => OfflinePackets::OpenConnectReply,
               0x07 => OfflinePackets::SessionInfo,
               0x08 => OfflinePackets::SessionInfoReply,
               0x1c => OfflinePackets::UnconnectedPong,
               _ => OfflinePackets::UnknownPacket
          }
     }
}

/// Open Connection Reply
/// Sent to the client when the server accepts a client.
pub struct OpenConnectReply {
     server_id: i64,
     security: bool,
     mtu: u16,
}

impl IClientBound<OpenConnectReply> for OpenConnectReply {
     fn to(&self) -> BinaryStream {
         let mut stream = BinaryStream::new();
         stream.write_byte(OfflinePackets::OpenConnectReply as u16);
         stream.write_magic();
         stream.write_long(self.server_id);
         stream.write_bool(self.security);
         stream.write_short(self.mtu);
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
         let p = s.read_byte();
         let mtu = s.get_length() + 1 + 28;
         OpenConnectRequest {
             protocol: p,
             mtu_size: mtu,
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
     magic: Vec<u8>,
     server_id: i64,
     motd: Motd,
}

impl IClientBound<UnconnectedPong> for UnconnectedPong {
      fn to(&self) -> BinaryStream {
          let mut stream = BinaryStream::new();
          stream.write_long(self.timestamp);
          stream.write_magic();
          stream.write_long(self.server_id);
          stream.write_string(self.motd.parse());
          return stream;
      }
}

/// Session info, also known as Open Connect Request 2
pub struct SessionInfo {
     magic: Vec<u8>,
     address: SocketAddr,
     mtu: usize,
     client_id: i64,
}

impl IServerBound<SessionInfo> for SessionInfo {
     fn recv(mut stream: BinaryStream) -> SessionInfo {
         Self {
             magic: stream.read_magic(),
             address: stream.read_address(),
             mtu: stream.read_short() as usize,
             client_id: stream.read_long(),
         }
     }
}

/// Session Info Reply, also known as Open Connect Reply 2
pub struct SessionInfoReply {
     magic: Vec<u8>,
     server_id: i64,
     client_id: i64,
     mtu: usize,
     security: bool,
}

impl IClientBound<SessionInfoReply> for SessionInfoReply {
     fn to(&self) -> BinaryStream {
         let mut stream: BinaryStream = BinaryStream::new();
         stream.write_byte(OfflinePackets::SessionInfoReply as u16);
         stream.write_magic();
         stream.write_long(self.server_id);
         stream.write_long(self.client_id);
         stream.write_usize(self.mtu);
         stream.write_bool(self.security);
         stream
    }
}

pub fn handle_pong(connection: &mut Connection, stream: &mut BinaryStream) {
     let rx = UnconnectedPing::recv(stream.clone());
     let rs = UnconnectedPong {
          magic: MAGIC.to_vec(),
          server_id: SERVER_ID,
          timestamp: rx.timestamp,
          motd: connection.gen_motd()
     };

     connection.send_queue.push_front(rs.to());
}