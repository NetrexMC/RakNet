use crate::SERVER_ID;
use binary_utils::BinaryStream;
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
     pub id: u16,
}

#[derive(Clone, Debug)]
pub struct Motd {
     pub name: String,
     pub protocol: u16,
     pub version: String,
     pub player_count: u16,
     pub player_max: u16,
     pub gamemode: String,
     pub server_id: i64,
}

impl Motd {
     pub fn default() -> Self {
          Self {
               name: String::from("Netrex Server"),
               player_count: 10,
               player_max: 100,
               protocol: 448,
               gamemode: String::from("Creative"),
               version: String::from("1.17.10"),
               server_id: SERVER_ID,
          }
     }

     pub fn parse(&self) -> String {
          let mut parsed = String::new();
          let prot = self.protocol.to_string();
          let pcount = self.player_count.to_string();
          let pmax = self.player_max.to_string();
          let server_id = SERVER_ID.to_string();
          let props = vec![
               "MCPE",
               self.name.as_str(),
               prot.as_str(),
               self.version.as_str(),
               pcount.as_str(),
               pmax.as_str(),
               server_id.as_str(),
               "Netrex",
               self.gamemode.as_str(),
               "1",
               "19132",
               "19133",
          ];

          for prop in props.iter() {
               parsed.push_str(prop);
               parsed.push(';');
          }

          parsed
     }
}
