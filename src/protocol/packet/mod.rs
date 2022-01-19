/// Handlers for both online & offline packets!
/// This is used by the connection struct to handle packets.
pub(crate) mod handler;

/// The protocol that is used when a client is considered "Online".
/// Sessions in this state are usually: Connected or Connecting
pub mod online;

/// The protocol that is used when a client is considered "Unidentified".
/// This module is used for Pong and connection requests.
pub mod offline;

use std::io::Write;

use binary_utils::Streamable;
use byteorder::WriteBytesExt;

use self::offline::{
    IncompatibleProtocolVersion, OpenConnectReply, OpenConnectRequest, SessionInfoReply,
    SessionInfoRequest, UnconnectedPing, UnconnectedPong,
};
use self::online::{
    ConnectedPing, ConnectedPong, ConnectionAccept, ConnectionRequest, Disconnect, LostConnection,
    NewConnection,
};

use super::offline::OfflinePacket;
use super::online::OnlinePacket;

/// A helper trait to identify packets.
pub trait PacketId {
    fn id() -> u8;

    fn get_id(&self) -> u8 {
        Self::id()
    }
}

/// A Generic Packet.
/// This is the base for all packets.
#[derive(Clone, Debug)]
pub struct Packet {
    /// The packet id.
    pub id: u8,
    /// The packet data. (this is the payload)
    pub payload: Payload,
}

impl Packet {
    pub fn is_online(&self) -> bool {
        match self.payload {
            Payload::Online(_) => true,
            _ => false,
        }
    }

    pub fn is_offline(&self) -> bool {
        return !self.is_online();
    }

    pub fn get_online(&self) -> OnlinePacket {
        self.clone().into()
    }

    pub fn get_offline(&self) -> OfflinePacket {
        self.clone().into()
    }
}

impl Streamable for Packet {
    fn compose(
        source: &[u8],
        position: &mut usize,
    ) -> Result<Self, binary_utils::error::BinaryError> {
        let payload = Payload::compose(source, position)?;
        let id = source[0];
        Ok(Packet { id, payload })
    }

    fn parse(&self) -> Result<Vec<u8>, binary_utils::error::BinaryError> {
        let mut buffer: Vec<u8> = Vec::new();
        buffer.write_u8(self.id)?;
        buffer.write_all(&self.payload.parse()?)?;
        Ok(buffer)
    }
}

#[derive(Clone, Debug)]
pub enum Payload {
    Online(OnlinePacket),
    Offline(OfflinePacket),
}

impl Streamable for Payload {
    fn compose(
        source: &[u8],
        position: &mut usize,
    ) -> Result<Self, binary_utils::error::BinaryError> {
        // we need the id!
        let id = u8::compose(source, position)?;

        match id {
            x if x == UnconnectedPing::id() => {
                let packet =
                    OfflinePacket::UnconnectedPing(UnconnectedPing::compose(source, position)?);
                Ok(Payload::Offline(packet))
            }
            x if x == UnconnectedPong::id() => {
                let packet =
                    OfflinePacket::UnconnectedPong(UnconnectedPong::compose(source, position)?);
                Ok(Payload::Offline(packet))
            }
            x if x == OpenConnectRequest::id() => {
                let packet = OfflinePacket::OpenConnectRequest(OpenConnectRequest::compose(
                    source, position,
                )?);
                Ok(Payload::Offline(packet))
            }
            x if x == OpenConnectReply::id() => {
                let packet =
                    OfflinePacket::OpenConnectReply(OpenConnectReply::compose(source, position)?);
                Ok(Payload::Offline(packet))
            }
            x if x == SessionInfoRequest::id() => {
                let packet = OfflinePacket::SessionInfoRequest(SessionInfoRequest::compose(
                    source, position,
                )?);
                Ok(Payload::Offline(packet))
            }
            x if x == SessionInfoReply::id() => {
                let packet =
                    OfflinePacket::SessionInfoReply(SessionInfoReply::compose(source, position)?);
                Ok(Payload::Offline(packet))
            }
            x if x == IncompatibleProtocolVersion::id() => {
                let packet = OfflinePacket::IncompatibleProtocolVersion(
                    IncompatibleProtocolVersion::compose(source, position)?,
                );
                Ok(Payload::Offline(packet))
            }
            x if x == ConnectedPing::id() => {
                let packet = OnlinePacket::ConnectedPing(ConnectedPing::compose(source, position)?);
                Ok(Payload::Online(packet))
            }
            x if x == ConnectedPong::id() => {
                let packet = OnlinePacket::ConnectedPong(ConnectedPong::compose(source, position)?);
                Ok(Payload::Online(packet))
            }
            x if x == LostConnection::id() => {
                let packet =
                    OnlinePacket::LostConnection(LostConnection::compose(source, position)?);
                Ok(Payload::Online(packet))
            }
            x if x == ConnectionRequest::id() => {
                let packet =
                    OnlinePacket::ConnectionRequest(ConnectionRequest::compose(source, position)?);
                Ok(Payload::Online(packet))
            }
            x if x == ConnectionAccept::id() => {
                let packet =
                    OnlinePacket::ConnectionAccept(ConnectionAccept::compose(source, position)?);
                Ok(Payload::Online(packet))
            }
            x if x == NewConnection::id() => {
                let packet = OnlinePacket::NewConnection(NewConnection::compose(source, position)?);
                Ok(Payload::Online(packet))
            }
            x if x == Disconnect::id() => {
                let packet = OnlinePacket::Disconnect(Disconnect::compose(source, position)?);
                Ok(Payload::Online(packet))
            }
            _ => Err(binary_utils::error::BinaryError::RecoverableKnown(format!(
                "Id is not a valid raknet packet: {}",
                id
            ))),
        }
    }

    fn parse(&self) -> Result<Vec<u8>, binary_utils::error::BinaryError> {
        let mut buffer: Vec<u8> = Vec::new();
        // we're not concerned about the id here so we're going to only write the payload!
        let payload = match self {
            Payload::Online(packet) => match packet {
                OnlinePacket::ConnectedPing(pk) => pk.parse()?,
                OnlinePacket::ConnectedPong(pk) => pk.parse()?,
                OnlinePacket::LostConnection(pk) => pk.parse()?,
                OnlinePacket::ConnectionRequest(pk) => pk.parse()?,
                OnlinePacket::ConnectionAccept(pk) => pk.parse()?,
                OnlinePacket::NewConnection(pk) => pk.parse()?,
                OnlinePacket::Disconnect(pk) => pk.parse()?,
            },
            Payload::Offline(packet) => match packet {
                OfflinePacket::UnconnectedPing(pk) => pk.parse()?,
                OfflinePacket::UnconnectedPong(pk) => pk.parse()?,
                OfflinePacket::OpenConnectRequest(pk) => pk.parse()?,
                OfflinePacket::OpenConnectReply(pk) => pk.parse()?,
                OfflinePacket::SessionInfoRequest(pk) => pk.parse()?,
                OfflinePacket::SessionInfoReply(pk) => pk.parse()?,
                OfflinePacket::IncompatibleProtocolVersion(pk) => pk.parse()?,
            },
        };
        if let Err(_) = buffer.write_all(&payload) {
            Err(binary_utils::error::BinaryError::RecoverableKnown(
                "Failed to write payload to buffer".to_string(),
            ))
        } else {
            Ok(buffer)
        }
    }
}

/// This allows implemenation for:
/// ```rust no_run
/// use raknet::packet::Packet;
/// use raknet::packet::online::OnlinePacket;
/// let online: OnlinePacket = Packet::compose(source, position)?;
/// ```
impl From<Packet> for OnlinePacket {
    fn from(packet: Packet) -> Self {
        match packet.payload {
            Payload::Online(x) => x,
            _ => panic!("This is not an online packet!"),
        }
    }
}

/// This allows implemenation for:
/// ```rust no_run
/// use raknet::packet::Packet;
/// use raknet::packet::offline::OfflinePacket;
/// let offline: OfflinePacket = Packet::compose(source, position)?;
/// ```
impl From<Packet> for OfflinePacket {
    fn from(packet: Packet) -> Self {
        match packet.payload {
            Payload::Offline(x) => x,
            _ => panic!("This is not an online packet!"),
        }
    }
}

/// This implementation allows for conversion of a OnlinePacket to a payload.
/// This isn't really used externally, but rather internally within the `Packet` struct.
/// ```rust no_run
/// use raknet::packet::Packet;
/// use raknet::packet::online::OnlinePacket;
/// let payload: Payload = OnlinePacket(_).into();
/// ```
impl From<OnlinePacket> for Payload {
    fn from(packet: OnlinePacket) -> Self {
        Payload::Online(packet)
    }
}

/// This implementation allows for conversion of a OfflinePacket to a payload.
/// This isn't really used externally, but rather internally within the `Packet` struct.
/// ```rust no_run
/// use raknet::packet::Packet;
/// use raknet::packet::offline::OfflinePacket;
/// let payload: Payload = OfflinePacket(_).into();
/// ```
impl From<OfflinePacket> for Payload {
    fn from(packet: OfflinePacket) -> Self {
        Payload::Offline(packet)
    }
}

/// A utility macro to add the `PacketId` trait to a packet.
/// This allows easier decoding of that packet.
#[macro_export]
macro_rules! packet_id {
    ($name: ident, $id: literal) => {
        impl PacketId for $name {
            fn id() -> u8 {
                $id
            }
        }
    };
}

/// A utility macro that adds the implementation for any `OnlinePacket(Pk)` where
/// `Pk` can be converted to `Packet`, `Payload`, `OnlinePacket` or `OfflinePacket`
/// and vice versa.
///
/// For example, we want unconnected pong to be unwrapped, we can do this without
/// a match statement like this:
/// ```rust no_run
/// use raknet::packet::Packet;
/// use raknet::packet::online::OnlinePacket;
/// use raknet::packet::online::UnconnectedPing;
/// let some_packet = Packet::compose(&source, &mut position)?;
/// let connected_ping: UnconnectedPing = some_packet.into();
/// ```
///
/// This macro also allows for converting any `OnlinePacket(Pk)` to a `Packet`, where `Pk` can
/// be directly converted into a packet. For example:
/// ```rust no_run
/// use raknet::packet::Packet;
/// use raknet::packet::online::OnlinePacket;
/// use raknet::packet::online::UnconnectedPong;
///
/// let packet: Packet = UnconnectedPong {
///     magic: Magic::new(),
///     timestamp: SystemTime::now(),
///     client_id: -129
/// }.into();
/// ```
#[macro_export]
macro_rules! register_packets {
    ($name: ident is $kind: ident, $($packet: ident),*) => {
        $(
            impl From<$packet> for $kind {
                fn from(packet: $packet) -> Self {
                    $kind::$packet(packet)
                }
            }

            impl From<$kind> for $packet {
                fn from(packet: $kind) -> Self {
                    match packet {
                        $kind::$packet(packet) => packet,
                        _ => panic!("Invalid packet type")
                    }
                }
            }

            impl From<$packet> for Packet {
                fn from(payload: $packet) -> Self {
                    Self {
                        id: payload.get_id(),
                        payload: Payload::$name(payload.into()),
                    }
                }
            }

            impl From<$packet> for Payload {
                fn from(payload: $packet) -> Self {
                    Self::$name(payload.into())
                }
            }

            impl From<Payload> for $packet {
                fn from(payload: Payload) -> Self {
                    match payload {
                        Payload::$name(v) => v.into(),
                        _ => panic!("Invalid payload type"),
                    }
                }
            }
        )*
    };
}
