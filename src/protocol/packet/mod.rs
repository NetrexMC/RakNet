// /// Handlers for both online & offline packets!
// /// This is used by the connection struct to handle packets.
// pub(crate) mod handler;

/// The protocol that is used when a client is considered "Online".
/// Sessions in this state are usually: Connected or Connecting
pub mod online;

/// The protocol that is used when a client is considered "Unidentified".
/// This module is used for Pong and connection requests.
pub mod offline;

use binary_util::interfaces::{Reader, Writer};

use self::offline::OfflinePacket;
use self::online::OnlinePacket;

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
