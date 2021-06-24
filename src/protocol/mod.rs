use binary_utils::BinaryStream;
pub mod offline;
pub mod online;

/// Trait used when a packet should be **sent** to the client.
/// AKA: This packet is being sent to the client.
pub trait IClientBound<Pk = Packet> {
     fn to(packet: Pk) -> BinaryStream;
}

/// Trait used when a packet is being **recieved** from the client.
/// AKA: This packet is being recieved from the client.
pub trait IServerBound<Pk = Packet> {
     fn from(stream: BinaryStream) -> Pk;
}

pub struct Packet {
     pub stream: BinaryStream,
     pub id: u16
}