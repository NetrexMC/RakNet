use binary_utils::BinaryStream;
pub mod offline;
pub mod online;

pub trait IClientBound<Pk = Packet> {
     fn to(packet: Pk) -> BinaryStream;
}

pub trait IServerBound<Pk = Packet> {
     fn from(stream: BinaryStream) -> Pk;
}

pub struct Packet {
     pub stream: BinaryStream,
     pub id: u16
}