//! This module contains all the packets that are used by the RakNet protocol.
//! This module is split into two submodules:
//! - [`offline`]: Any packet that is not sent within a [`Frame`].
//! - [`online`]: Any packet considered to be online, which is sent within a [`Frame`].
//!
//! [`offline`]: crate::protocol::packet::offline
//! [`online`]: crate::protocol::packet::online
// /// Handlers for both online & offline packets!
// /// This is used by the connection struct to handle packets.
// pub(crate) mod handler;

pub mod offline;
pub mod online;

use binary_util::interfaces::{Reader, Writer};

use self::offline::OfflinePacket;
use self::online::OnlinePacket;

/// A wrapper or helper for both online and offline packets.
/// This allows for a single type to be read with `Reader` and written with `Writer`,
/// traits provided by `binary_util`.
///
/// All packets sent are wrapped in this type, and can be unwrapped with the `get_offline` or `get_online`
#[derive(Debug, Clone)]
pub enum RakPacket {
    Offline(OfflinePacket),
    Online(OnlinePacket),
}

impl RakPacket {
    pub fn is_online(&self) -> bool {
        match self {
            RakPacket::Online(_) => true,
            _ => false,
        }
    }

    pub fn get_offline(&self) -> Option<&OfflinePacket> {
        match self {
            RakPacket::Offline(packet) => Some(packet),
            _ => None,
        }
    }

    pub fn get_online(&self) -> Option<&OnlinePacket> {
        match self {
            RakPacket::Online(packet) => Some(packet),
            _ => None,
        }
    }
}

impl Writer for RakPacket {
    fn write(&self, buf: &mut binary_util::io::ByteWriter) -> Result<(), std::io::Error> {
        match self {
            RakPacket::Offline(packet) => packet.write(buf),
            RakPacket::Online(packet) => packet.write(buf),
        }
    }
}

impl Reader<RakPacket> for RakPacket {
    fn read(buf: &mut binary_util::ByteReader) -> Result<RakPacket, std::io::Error> {
        if let Ok(packet) = OfflinePacket::read(buf) {
            return Ok(RakPacket::Offline(packet));
        }
        if let Ok(packet) = OnlinePacket::read(buf) {
            return Ok(RakPacket::Online(packet));
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid packet",
        ))
    }
}

impl From<OfflinePacket> for RakPacket {
    fn from(packet: OfflinePacket) -> Self {
        RakPacket::Offline(packet)
    }
}

impl From<OnlinePacket> for RakPacket {
    fn from(packet: OnlinePacket) -> Self {
        RakPacket::Online(packet)
    }
}

impl From<RakPacket> for OnlinePacket {
    fn from(packet: RakPacket) -> Self {
        match packet {
            RakPacket::Online(packet) => packet,
            _ => panic!("Invalid packet conversion"),
        }
    }
}

impl From<RakPacket> for OfflinePacket {
    fn from(packet: RakPacket) -> Self {
        match packet {
            RakPacket::Offline(packet) => packet,
            _ => panic!("Invalid packet conversion"),
        }
    }
}

/// A utility macro that adds the implementation for any `OnlinePacket(Pk)` where
/// `Pk` can be converted to `RakPacket`, `OnlinePacket` or `OfflinePacket`
/// and vice versa.
///
/// For example, we want unconnected pong to be unwrapped, we can do this without
/// a match statement like this:
/// ```rust ignore
/// use raknet::packet::RakPacket;
/// use raknet::packet::online::OnlinePacket;
/// use raknet::packet::online::UnconnectedPing;
/// let some_packet = RakPacket::from(&source)?;
/// let connected_ping: UnconnectedPing = some_packet.into();
/// ```
///
/// This macro also allows for converting any `OnlinePacket(Pk)` to a `RakPacket`, where `Pk` can
/// be directly converted into a packet. For example:
/// ```rust ignore
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
///
/// The macro can be expressed in the following way:
/// ```rust ignore
/// register_packets! {
///     Online is OnlinePacket,
///     UnconnectedPing,
///     // etc...
/// }
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

            impl From<$packet> for RakPacket {
                fn from(packet: $packet) -> Self {
                    $kind::$packet(packet).into()
                }
            }

            impl From<$kind> for $packet {
                fn from(packet: $kind) -> Self {
                    match packet {
                        $kind::$packet(packet) => packet.into(),
                        _ => panic!("Invalid packet conversion"),
                    }
                }
            }

            impl From<RakPacket> for $packet {
                fn from(packet: RakPacket) -> Self {
                    match packet {
                        RakPacket::$name(packet) => packet.into(),
                        _ => panic!("Invalid packet conversion"),
                    }
                }
            }
        )*
    };
}
