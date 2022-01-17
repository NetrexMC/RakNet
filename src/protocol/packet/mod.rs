/// The protocol that is used when a client is considered "Online".
/// Sessions in this state are usually: Connected or Connecting
pub mod online;

/// The protocol that is used when a client is considered "Unidentified".
/// This module is used for Pong and connection requests.
pub mod offline;

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

#[derive(Clone, Debug)]
pub enum Payload {
    Online(OnlinePacket),
    Offline(OfflinePacket),
}

// Online Packets -> Payload
impl From<OnlinePacket> for Payload {
    fn from(packet: OnlinePacket) -> Self {
        Payload::Online(packet)
    }
}

/// Offline Packets -> Payload
impl From<OfflinePacket> for Payload {
    fn from(packet: OfflinePacket) -> Self {
        Payload::Offline(packet)
    }
}

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
