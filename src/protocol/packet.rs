use binary_utils::*;

use super::online::OnlinePacket;
use super::offline::OfflinePacket;

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
    pub payload: Payload
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
macro_rules! register_online_packets {
    ($($packet: ident),*) => {
        $(
            impl From<$packet> for OnlinePacket {
                fn from(packet: $packet) -> Self {
                    OnlinePacket::$packet(packet)
                }
            }

            impl From<OnlinePacket> for $packet {
                fn from(packet: OnlinePacket) -> Self {
                    match packet {
                        OnlinePacket::$packet(packet) => packet,
                        _ => panic!("Invalid packet type")
                    }
                }
            }

            impl From<$packet> for Packet {
                fn from(payload: $packet) -> Self {
                    Self {
                        id: payload.get_id(),
                        payload: Payload::Online(payload.into()),
                    }
                }
            }

            impl From<$packet> for Payload {
                fn from(payload: $packet) -> Self {
                    Self::Online(payload.into())
                }
            }

            impl From<Payload> for $packet {
                fn from(payload: Payload) -> Self {
                    match payload {
                        Payload::Online(v) => v.into(),
                        _ => panic!("Invalid payload type"),
                    }
                }
            }
        )*
    };
}
