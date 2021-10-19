#![allow(dead_code)]
use crate::conn::{Connection, ConnectionState};
use crate::util::tokenize_addr;
use crate::RakNetEvent;
use binary_utils::*;
use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};
use std::io::{Read, Write};
use std::fmt::{Formatter, Result as FResult};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::SystemTime;

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
               0x80..=0x8d => OnlinePackets::FramePacket(byte),
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
               OnlinePackets::FramePacket(byte) => write!(f, "{}", byte),
          }
     }
}

#[derive(Debug, BinaryStream)]
pub struct ConnectionRequest {
     client_id: i64,
     timestamp: i64,
}

#[derive(Debug)]
pub struct ConnectionAccept {
     id: u8,
     client_address: SocketAddr,
     system_index: i16,
     internal_ids: SocketAddr,
     request_time: i64,
     timestamp: i64,
}

impl Streamable for ConnectionAccept {
     fn parse(&self) -> Vec<u8> {
         let mut stream = Vec::new();
         stream.write_u8(self.id).unwrap();
         stream.write_all(&self.client_address.parse()[..]).unwrap();
         stream.write_i16::<BigEndian>(self.system_index).unwrap();
         for _ in 0..10 {
              stream.write_all(&self.internal_ids.parse()[..]).unwrap();
         }
         stream.write_i64::<BigEndian>(self.request_time).unwrap();
         stream.write_i64::<BigEndian>(self.timestamp).unwrap();
         stream
     }

     fn compose(_source: &[u8], _position: &mut usize) -> Self {
          Self {
               id: 0,
               client_address: SocketAddr::new(IpAddr::from(Ipv4Addr::new(192,168,0,1)), 9120),
               system_index: 0,
               internal_ids: SocketAddr::new(IpAddr::from(Ipv4Addr::new(127,0,0,1)), 1920),
               request_time: 0,
               timestamp: 0
          }
     }
}

#[derive(Debug, BinaryStream)]
pub struct ConnectedPing {
     time: i64,
}

#[derive(Debug, BinaryStream)]
pub struct ConnectedPong {
     id: u8,
     ping_time: i64,
     pong_time: i64,
}

pub fn handle_online(
     connection: &mut Connection,
     pk: OnlinePackets,
     stream: &mut Vec<u8>,
) -> Vec<u8> {
     match pk {
          OnlinePackets::ConnectionRequest => {
               let request = ConnectionRequest::compose(stream, &mut 1);
               let accept = ConnectionAccept {
                    id: OnlinePackets::ConnectionAccept.to_byte(),
                    client_address: connection.address.clone(),
                    system_index: 0,
                    internal_ids: SocketAddr::new(
                         IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)),
                         19132,
                    ),
                    request_time: request.timestamp,
                    timestamp: SystemTime::now()
                         .duration_since(connection.time)
                         .unwrap()
                         .as_millis() as i64,
               };
               connection.state = ConnectionState::Connected;
               accept.parse()
          }
          OnlinePackets::Disconnect => {
               connection.state = ConnectionState::Offline;
               connection.event_dispatch.push_back(RakNetEvent::Disconnect(
                    tokenize_addr(connection.address),
                    "Client disconnect".to_owned(),
               ));
               Vec::new()
          }
          OnlinePackets::NewConnection => Vec::new(),
          OnlinePackets::ConnectedPing => {
               let request = ConnectedPing::compose(stream, &mut 0);
               let pong = ConnectedPong {
                    id: OnlinePackets::ConnectedPong.to_byte(),
                    ping_time: request.time,
                    pong_time: SystemTime::now()
                         .duration_since(connection.time)
                         .unwrap()
                         .as_millis() as i64,
               };
               pong.parse()
          }
          OnlinePackets::FramePacket(_v) => {
               println!("Condition should never be met.");
               Vec::new()
          }
          _ => Vec::new(), // TODO: Throw an UnknownPacket here rather than sending an empty binary stream
     }
}
