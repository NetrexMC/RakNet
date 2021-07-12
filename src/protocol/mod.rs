use binary_utils::BinaryStream;
use crate::SERVER_ID;
pub mod offline;
pub mod online;

/// Trait used when a packet should be **sent** to the client.
/// AKA: This packet is being sent to the client.
pub trait IClientBound<Pk = Packet> {
     fn to(&self) -> BinaryStream;
}

/// Trait used when a packet is being **recieved** from the client.
/// AKA: This packet is being recieved from the client.
pub trait IServerBound<Pk = Packet> {
     fn recv(stream: BinaryStream) -> Pk;
}

pub struct Packet {
     pub stream: BinaryStream,
     pub id: u16
}

pub struct Motd {
     pub name: String,
     pub protocol: u8,
     pub version: String,
     pub player_count: u16,
     pub player_max: u16,
     pub gamemode: String,
     pub server_id: i64
}

impl Motd {
     pub fn parse(&self) -> String {
          let mut parsed = String::new();
          let props = vec![
               "MCPE",
               "Netrex",
               self.protocol.to_string().as_str(),
               self.version.as_str(),
               self.player_count.to_string().as_str(),
               self.player_max.to_string().as_str(),
               SERVER_ID.to_string().as_str(),
               self.name.as_str(),
               self.gamemode.as_str()
          ];

          for prop in props.iter() {
               parsed.push_str(prop);
               parsed.push(';');
          }

          parsed
     }
}