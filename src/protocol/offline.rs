use super::{ IClientBound, IServerBound, Packet };
use crate::IPacketStreamWrite;
use binary_utils::{ stream::*, IBufferRead, IBufferWrite };

pub enum OfflinePackets {
     UnconnectedPing = 0x01,
     OpenConnectRequest = 0x05,
     OpenConnectReply = 0x06,
     SessionInfo = 0x07,
     SessionInfoReply = 0x08,
     UnconnectedPong = 0x1c,
}

// Open Connection Reply One
pub struct OpenConnectReply {
     server_id: i64,
     security: bool,
     mtu: u16,
}

impl IClientBound<OpenConnectReply> for OpenConnectReply {
     fn to(packet: OpenConnectReply) -> BinaryStream {
          let mut stream = BinaryStream::new(vec![0]);
          stream.write_byte(OfflinePackets::OpenConnectReply as u16);
          stream.write_magic();
          stream.write_long(packet.server_id);
          stream.write_bool(packet.security);
          stream.write_short(packet.mtu);
          stream
     }
}
