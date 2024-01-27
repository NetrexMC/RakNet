//! Online packets are packets that are sent when the client is connected to the server
//! and are used to keep the connection alive, and to send game packets to the server.
//!
//! The following module provides the following packets:
//! - [`ConnectedPing`]
//! - [`ConnectedPong`]
//! - [`LostConnection`]
//! - [`ConnectionRequest`]
//! - [`ConnectionAccept`]
//! - [`NewConnection`]
//! - [`Disconnect`]
//!
//! During this stage, the client and server are exchanging information about each other,
//! to initialize the connection within raknet, and completing the connection handshake.
use std::net::SocketAddr;

use super::RakPacket;
use crate::register_packets;

use binary_util::interfaces::{Reader, Writer};
use binary_util::io::{ByteReader, ByteWriter};
use binary_util::BinaryIo;

/// An enum of all Online packets.
///
/// You can use this to read and write online packets,
/// with the `binary_util` traits `Reader` and `Writer`.
#[derive(BinaryIo, Clone, Debug)]
#[repr(u8)]
pub enum OnlinePacket {
    ConnectedPing(ConnectedPing) = 0x00,
    ConnectedPong(ConnectedPong) = 0x03,
    LostConnection(LostConnection) = 0x04,
    ConnectionRequest(ConnectionRequest) = 0x09,
    ConnectionAccept(ConnectionAccept) = 0x10,
    NewConnection(NewConnection) = 0x13,
    Disconnect(Disconnect) = 0x15,
}

register_packets! {
    Online is OnlinePacket,
    ConnectedPing,
    ConnectedPong,
    LostConnection,
    ConnectionRequest,
    ConnectionAccept,
    NewConnection,
    Disconnect
}

/// This packet is sent by either the client or the server to the other peer.
/// The other peer should respond with a [`ConnectedPong`] packet. In general
/// you should be sending this packet every 5 seconds to keep the connection alive.
/// <br />
/// <br />
/// If you do not continue to send this packet, the connection will be closed after
/// the other peer does not receive a [`ConnectedPong`] packet for the configured timeout option.
#[derive(Clone, Debug, BinaryIo)]
pub struct ConnectedPing {
    /// The time you sent the packet to the peer.
    pub time: i64,
}

/// Sent in response to a [`ConnectedPing`] packet.
///
/// This packet is sent by the other peer in response to a [`ConnectedPing`] packet as
/// an acknowledgement that the connection is still alive. It contains the time of the
/// round trip from the time that the initiator sent the [`ConnectedPing`] packet.
#[derive(Clone, Debug, BinaryIo)]
pub struct ConnectedPong {
    /// The time that the peer sent the [`ConnectedPing`] packet.
    pub ping_time: i64,
    /// The time that you sent the [`ConnectedPong`] packet to the peer.
    pub pong_time: i64,
}

/// A connection Request Request, this contains information about the client. Like it's
/// current time and the client id.
#[derive(Clone, Debug, BinaryIo)]
pub struct ConnectionRequest {
    pub client_id: i64,
    pub time: i64,
    pub security: bool,
}

/// A connection Accept packet, this is sent by the server to the client.
/// This is sent by the server and contains information about the server.
#[derive(Clone, Debug)]
pub struct ConnectionAccept {
    /// The address of the client connecting (locally?).
    pub client_address: SocketAddr,
    /// The system index of the server.
    pub system_index: i16,
    /// The internal id's of the server or alternative IP's of the server.
    /// These are addresses the client will use if it can't connect to the server.
    /// (Not sure why this is useful)
    pub internal_ids: Vec<SocketAddr>,
    /// The time of the timestamp the client sent with `ConnectionRequest`.
    pub request_time: i64,
    /// The time on the server.
    pub timestamp: i64,
}

impl Reader<ConnectionAccept> for ConnectionAccept {
    fn read(buf: &mut ByteReader) -> std::io::Result<Self> {
        let client_address = buf.read_type::<SocketAddr>()?;

        // read the system index, this is
        let system_index = buf.read_i16()?;
        let mut internal_ids = Vec::<SocketAddr>::new();

        for _ in 0..20 {
            // we only have the request time and timestamp left...
            if buf.as_slice().len() <= 16 {
                break;
            }
            internal_ids.push(buf.read_type::<SocketAddr>()?);
        }

        let request_time = buf.read_i64()?;
        let timestamp = buf.read_i64()?;

        Ok(Self {
            client_address,
            system_index,
            internal_ids,
            request_time,
            timestamp,
        })
    }
}

impl Writer for ConnectionAccept {
    fn write(&self, buf: &mut ByteWriter) -> std::io::Result<()> {
        buf.write_type::<SocketAddr>(&self.client_address)?;
        buf.write_i16(self.system_index)?;

        if self.internal_ids.len() > 20 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Too many internal id's",
            ));
        }

        for internal_id in &self.internal_ids {
            buf.write_type::<SocketAddr>(internal_id)?;
        }

        buf.write_i64(self.request_time)?;
        buf.write_i64(self.timestamp)?;

        Ok(())
    }
}

/// Going to be completely Honest here, I have no idea what this is used for right now,
/// even after reading the source code.
#[derive(Clone, Debug)]
pub struct NewConnection {
    /// The external IP Address of the server.
    pub server_address: SocketAddr,
    /// The internal IP Address of the server.
    pub system_address: Vec<SocketAddr>,
    /// The time of the timestamp the client sent with `ConnectionRequest`.
    pub request_time: i64,
    /// The time on the server.
    pub timestamp: i64,
}

impl Reader<NewConnection> for NewConnection {
    fn read(buf: &mut ByteReader) -> std::io::Result<Self> {
        let server_address = buf.read_type::<SocketAddr>()?;

        let mut system_address = Vec::<SocketAddr>::new();

        for _ in 0..20 {
            // we only have the request time and timestamp left...
            if buf.as_slice().len() < 16 {
                break;
            }
            system_address.push(buf.read_type::<SocketAddr>()?);
        }

        let request_time = buf.read_i64()?;
        let timestamp = buf.read_i64()?;

        Ok(Self {
            server_address,
            system_address,
            request_time,
            timestamp,
        })
    }
}

impl Writer for NewConnection {
    fn write(&self, buf: &mut ByteWriter) -> std::io::Result<()> {
        buf.write_type::<SocketAddr>(&self.server_address)?;

        if self.system_address.len() > 20 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Too many internal id's",
            ));
        }

        for system_address in &self.system_address {
            buf.write_type::<SocketAddr>(system_address)?;
        }

        buf.write_i64(self.request_time)?;
        buf.write_i64(self.timestamp)?;

        Ok(())
    }
}

/// A disconnect notification. Tells the client to disconnect.
#[derive(Clone, Debug, BinaryIo)]
pub struct Disconnect {}

/// A connection lost notification.
/// This is sent by the client when it loses connection to the server.
#[derive(Clone, Debug, BinaryIo)]
pub struct LostConnection {}
