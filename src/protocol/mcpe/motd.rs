use binary_util::interfaces::{Reader, Writer};
use binary_util::io::{ByteReader, ByteWriter};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gamemode {
    Survival = 0,
    Creative,
    Adventure,
    Spectator,
}

impl Gamemode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Gamemode::Survival => "Survival",
            Gamemode::Creative => "Creative",
            Gamemode::Adventure => "Adventure",
            Gamemode::Spectator => "Spectator",
        }
    }
}

impl std::fmt::Display for Gamemode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = match *self {
            Gamemode::Survival => "0",
            Gamemode::Creative => "1",
            Gamemode::Adventure => "2",
            Gamemode::Spectator => "3",
        };
        write!(f, "{}", v)
    }
}

/// Protocol wise, motd is just a string
/// However we're using this struct to represent the motd
#[derive(Debug, Clone)]
pub struct Motd {
    /// The name of the server
    pub name: String,
    /// The protocol version
    pub protocol: u16,
    /// The version of the server
    pub version: String,
    /// The number of players online
    pub player_count: u32,
    /// The maximum number of players
    pub player_max: u32,
    /// The gamemode of the server
    pub gamemode: Gamemode,
    /// The server's GUID
    pub server_guid: u64,
    /// The server's port
    pub port: String,
    /// The IPv6 port
    // TODO: Implement this
    pub ipv6_port: String,
}

impl Motd {
    pub fn new<S: Into<String>>(server_guid: u64, port: S) -> Self {
        Self {
            name: "Netrex Server".into(),
            player_count: 10,
            player_max: 100,
            protocol: 448,
            gamemode: Gamemode::Survival,
            version: "1.18.0".into(),
            server_guid,
            port: port.into(),
            ipv6_port: "19133".into(),
        }
    }

    /// Takes the Motd and parses it into a valid MCPE
    /// MOTD buffer.
    pub fn write(&self) -> String {
        let props: Vec<String> = vec![
            "MCPE".into(),
            self.name.clone(),
            self.protocol.to_string(),
            self.version.clone(),
            self.player_count.to_string(),
            self.player_max.to_string(),
            self.server_guid.to_string(),
            "Netrex".to_string(),
            self.gamemode.as_str().to_string(),
            "1".to_string(),
            // TODO: Figure out why this is not working
            // self.gamemode.to_string(),
            self.port.to_string(),
            self.ipv6_port.to_string(),
        ];
        props.join(";").into()
    }
}

impl Reader<Motd> for Motd {
    fn read(buf: &mut ByteReader) -> Result<Motd, std::io::Error> {
        let str_len = buf.read_u16()?;
        let mut str_buf = vec![0; str_len as usize];

        buf.read(&mut str_buf)?;

        let motd = String::from_utf8(str_buf).unwrap();

        let parts = motd
            .split(";")
            .map(|c| c.to_string())
            .collect::<Vec<String>>();

        let name = parts
            .get(1)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid motd name",
            ))?
            .clone();

        let protocol = parts
            .get(2)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid motd protocol",
            ))?
            .clone();

        let version = parts
            .get(3)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid motd version",
            ))?
            .clone();

        let player_count = parts
            .get(4)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid motd player count",
            ))?
            .clone();

        let player_max = parts
            .get(5)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid motd player max",
            ))?
            .clone();

        let server_guid = parts
            .get(6)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid motd server guid",
            ))?
            .clone();

        let gamemode = parts
            .get(8)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid motd gamemode",
            ))?
            .clone();

        let port = parts
            .get(10)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid motd port",
            ))?
            .clone();

        let ipv6_port = parts
            .get(11)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid motd ipv6 port",
            ))?
            .clone();

        Ok(Motd {
            name,
            protocol: protocol.as_str().parse().unwrap(),
            version,
            player_count: player_count.parse().unwrap(),
            player_max: player_max.parse().unwrap(),
            server_guid: server_guid.parse().unwrap(),
            gamemode:  match gamemode
                .as_str()
                .parse::<u8>()
                .expect("Gamemode is not a byte")
            {
                0 => Gamemode::Survival,
                1 => Gamemode::Creative,
                2 => Gamemode::Adventure,
                3 => Gamemode::Spectator,
                _ => Gamemode::Survival,
            },
            port,
            ipv6_port,
        })
    }
}

impl Writer for Motd {
    fn write(&self, buf: &mut ByteWriter) -> Result<(), std::io::Error> {
        let motd = self.write();
        let motd_len = motd.len() as u16;
        buf.write_u16(motd_len)?;
        buf.write(motd.as_bytes())?;
        Ok(())
    }
}
