use std::io::Write;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

use binary_utils::*;
use binary_utils::error::BinaryError;
use byteorder::BigEndian;
use byteorder::WriteBytesExt;

use crate::{packet_id, register_online_packets};
use super::PacketId;
use super::Packet;
use super::Payload;

/// A enum that represents all online packets.
#[derive(Clone, Debug)]
pub enum OnlinePacket {
    ConnectedPing(ConnectedPing),
    ConnectedPong(ConnectedPong),
    ConnectionRequest(ConnectionRequest),
    ConnectionAccept(ConnectionAccept),
    NewConnection(NewConnection),
    Disconnect(Disconnect)
}

register_online_packets![
    ConnectedPing,
    ConnectedPong,
    ConnectionRequest,
    ConnectionAccept,
    NewConnection,
    Disconnect
];

/// Connected Ping Packet
/// This packet is sent by the client to the server.
/// The server should respond with a `ConnectedPong` packet.
#[derive(Clone, Debug, BinaryStream)]
pub struct ConnectedPing {
    pub time: i64
}
packet_id!(ConnectedPing, 0x00);

/// Connected Pong Packet
/// This packet is sent by the server to the client in response to a `ConnectedPing` packet.
#[derive(Clone, Debug, BinaryStream)]
pub struct ConnectedPong {
    pub ping_time: i64,
    pub pong_time: i64
}
packet_id!(ConnectedPong, 0x03);

/// A connection Request Request, this contains information about the client. Like it's
/// current time and the client id.
#[derive(Clone, Debug, BinaryStream)]
pub struct ConnectionRequest {
    pub client_id: i64,
    pub time: i64
}
packet_id!(ConnectionRequest, 0x09);

/// A connection Accept packet, this is sent by the server to the client.
/// This is sent by the server and contains information about the server.
#[derive(Clone, Debug)]
pub struct ConnectionAccept {
    /// The address of the client connecting (locally?).
    client_address: SocketAddr,
    /// The system index is the index of the system that the client is connected to.
    /// This is the index of the server on the client.
    /// (Not sure why this is useful)
    system_index: i16,
    /// The internal id's of the server or alternative IP's of the server.
    /// These are addresses the client will use if it can't connect to the server.
    /// (Not sure why this is useful)
    internal_id: SocketAddr,
    /// The time of the timestamp the client sent with `ConnectionRequest`.
    request_time: i64,
    /// The time on the server.
    timestamp: i64,
}

impl Streamable for ConnectionAccept {
    fn parse(&self) -> Result<Vec<u8>, BinaryError> {
        let mut stream = Vec::new();
        stream.write_all(&self.client_address.parse()?[..])?;
        stream.write_i16::<BigEndian>(self.system_index)?;
        for _ in 0..10 {
            stream.write_all(&self.internal_id.parse()?[..])?;
        }
        stream.write_i64::<BigEndian>(self.request_time)?;
        stream.write_i64::<BigEndian>(self.timestamp)?;
        Ok(stream)
    }

    fn compose(_source: &[u8], _position: &mut usize) -> Result<Self, BinaryError> {
        Ok(Self {
            client_address: SocketAddr::new(IpAddr::from(Ipv4Addr::new(192, 168, 0, 1)), 9120),
            system_index: 0,
            internal_id: SocketAddr::new(IpAddr::from(Ipv4Addr::new(127, 0, 0, 1)), 1920),
            request_time: 0,
            timestamp: 0,
        })
    }
}
packet_id!(ConnectionAccept, 0x10);

/// Going to be completely Honest here, I have no idea what this is used for right now,
/// even after reading the source code.
#[derive(Clone, Debug, BinaryStream)]
pub struct NewConnection {
    /// The external IP Address of the server.
    pub server_address: SocketAddr,
    /// The internal IP Address of the server.
    pub system_address: SocketAddr,
    /// The time of the timestamp the client sent with `ConnectionRequest`.
    pub request_time: i64,
    /// The time on the server.
    pub timestamp: i64,
}
packet_id!(NewConnection, 0x13);

/// A disconnect notification. Tells the client to disconnect.
#[derive(Clone, Debug, BinaryStream)]
pub struct Disconnect {}
packet_id!(Disconnect, 0x15);