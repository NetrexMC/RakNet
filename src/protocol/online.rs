use std::fmt::{Formatter, Result as FResult};
use crate::{IServerBound, IClientBound, IPacketStreamWrite};
use binary_utils::{BinaryStream, IBufferRead, IBinaryStream, IBufferWrite};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use crate::conn::Connection;
use crate::frame::*;
use std::time::{SystemTime, Duration};

#[derive(Debug, Clone, PartialEq)]
pub enum OnlinePackets {
     ConnectedPing,
     ConnectedPong,
     ConnectionRequest,
     ConnectionAccept,
     GamePacket,
     FramePacket(u8),
     NewConnection,
     Disconnect,
     UnknownPacket(u8),
}

impl OnlinePackets {
     pub fn recv(byte: u8) -> Self {
          match byte {
               0x00 => OnlinePackets::ConnectedPing,
               0x03 => OnlinePackets::ConnectedPong,
               0x09 => OnlinePackets::ConnectionRequest,
               0x10 => OnlinePackets::ConnectionAccept,
               0x13 => OnlinePackets::NewConnection,
               0x15 => OnlinePackets::Disconnect,
               0xfe => OnlinePackets::GamePacket,
               0x80..= 0x8d => OnlinePackets::FramePacket(byte),
               _ => OnlinePackets::UnknownPacket(byte),
          }
     }

     pub fn to_byte(&self) -> u8 {
          match *self {
               OnlinePackets::ConnectedPing => 0x00,
               OnlinePackets::ConnectedPong => 0x03,
               OnlinePackets::ConnectionRequest => 0x09,
               OnlinePackets::ConnectionAccept => 0x10,
               OnlinePackets::NewConnection => 0x13,
               OnlinePackets::Disconnect => 0x15,
               OnlinePackets::GamePacket => 0xfe,
               OnlinePackets::FramePacket(b) => b,
               OnlinePackets::UnknownPacket(byte) => byte,
          }
     }
}

impl std::fmt::Display for OnlinePackets {
     fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
          match *self {
               OnlinePackets::ConnectedPing => write!(f, "{}", self.to_byte()),
               OnlinePackets::ConnectedPong => write!(f, "{}", self.to_byte()),
               OnlinePackets::ConnectionRequest => write!(f, "{}", self.to_byte()),
               OnlinePackets::ConnectionAccept => write!(f, "{}", self.to_byte()),
               OnlinePackets::NewConnection => write!(f, "{}", self.to_byte()),
               OnlinePackets::Disconnect => write!(f, "{}", self.to_byte()),
               OnlinePackets::GamePacket => write!(f, "{}", self.to_byte()),
               OnlinePackets::UnknownPacket(byte) => write!(f, "{}", byte),
               OnlinePackets::FramePacket(byte) => write!(f, "{}", byte)
          }
     }
}

pub struct ConnectionRequest {
     client_id: i64,
     timestamp: i64,
}

impl IServerBound<ConnectionRequest> for ConnectionRequest {
     fn recv(mut stream: BinaryStream) -> ConnectionRequest {
          Self {
               client_id: stream.read_long(),
               timestamp: stream.read_long(),
          }
     }
}

pub struct ConnectionAccept {
     client_address: SocketAddr,
     system_index: i16,
     internal_ids: SocketAddr,
     request_time: i64,
     timestamp: i64,
}

impl IClientBound<ConnectionAccept> for ConnectionAccept {
     fn to(&self) -> BinaryStream {
          let mut stream = BinaryStream::new();
          stream.write_byte(OnlinePackets::ConnectionAccept.to_byte());
          stream.write_address(self.client_address);
          stream.write_short(self.system_index);
          for _ in 0..10 {
               stream.write_address(self.internal_ids);
          }
          stream.write_long(self.request_time);
          stream.write_long(self.timestamp);
          stream
     }
}

pub struct ConnectedPing {
     time: i64
}

impl IServerBound<ConnectedPing> for ConnectedPing {
     fn recv(mut stream: BinaryStream) -> ConnectedPing {
          ConnectedPing {
               time: stream.read_long()
          }
     }
}

pub struct ConnectedPong {
     ping_time: i64,
     pong_time: i64
}

impl IClientBound<ConnectedPong> for ConnectedPong {
     fn to(&self) -> BinaryStream {
          let mut stream = BinaryStream::new();
          stream.write_byte(OnlinePackets::ConnectedPong.to_byte());
          stream.write_long(self.ping_time);
          stream.write_long(SystemTime::now().elapsed().unwrap().as_millis() as i64);
          stream
     }
}

pub fn handle_online(
     connection: &mut Connection,
     pk: OnlinePackets,
     stream: &mut BinaryStream,
) -> BinaryStream {
     match pk {
          OnlinePackets::ConnectionRequest => {
               let request = ConnectionRequest::recv(stream.clone());
               let accept = ConnectionAccept {
                    client_address: connection.address.clone(),
                    system_index: 0,
                    internal_ids: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)), 19132),
                    request_time: request.timestamp,
                    timestamp: SystemTime::now().duration_since(connection.time).unwrap().as_millis() as i64,
               };
               accept.to()
          },
          OnlinePackets::NewConnection => {
               BinaryStream::new()
          },
          OnlinePackets::ConnectedPing => {
               let request = ConnectedPing::recv(stream.clone());
               println!("Responding to ping");
               let pong = ConnectedPong {
                    ping_time: request.time,
                    pong_time: 0
               };
               pong.to()
          }
          OnlinePackets::FramePacket(v) => {
               println!("Condition should never be met.");
               BinaryStream::new()
          },
          _ => BinaryStream::new(), // TODO: Throw an UnknownPacket here rather than sending an empty binary stream
     }
}