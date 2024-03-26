use binary_util::interfaces::{Reader, Writer};
use binary_util::io::{ByteReader, ByteWriter};
use std::str::FromStr;

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

impl FromStr for Gamemode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Survival" => Ok(Gamemode::Survival),
            "Creative" => Ok(Gamemode::Creative),
            "Adventure" => Ok(Gamemode::Adventure),
            "Spectator" => Ok(Gamemode::Spectator),
            _ => Err(format!("Invalid gamemode {}", s)),
        }
    }
}

/// Protocol wise, motd is just a string
/// However we're using this struct to represent the motd
#[derive(Debug, Clone)]
pub struct Motd {
    /// The edition of the server (MCPE or MCEE)
    pub edition: String,
    /// The name of the server
    pub name: String,
    /// The second line of the server MOTD
    pub sub_name: String,
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
    /// The server's IPv4 port
    pub port: Option<String>,
    /// The IPv6 port
    // TODO: Implement this
    pub ipv6_port: Option<String>,
    /// Is Nintendo limited
    pub nintendo_limited: Option<bool>,
}

impl Motd {
    pub fn new<S: Into<String>>(server_guid: u64, port: S) -> Self {
        Self {
            edition: "MCPE".into(),
            name: "Netrex Server".into(),
            sub_name: "Netrex".into(),
            player_count: 10,
            player_max: 100,
            protocol: 448,
            gamemode: Gamemode::Survival,
            version: "1.18.0".into(),
            server_guid,
            port: Some(port.into()),
            ipv6_port: Some("19133".into()),
            nintendo_limited: Some(false),
        }
    }

    /// Takes the Motd and parses it into a valid MCPE
    /// MOTD buffer.
    pub fn write(&self) -> String {
        let mut props: Vec<String> = vec![
            self.edition.clone(),
            self.name.clone(),
            self.protocol.to_string(),
            self.version.clone(),
            self.player_count.to_string(),
            self.player_max.to_string(),
            self.server_guid.to_string(),
            self.sub_name.clone(),
            self.gamemode.as_str().to_string(),
        ];

        if let Some(nintendo_limited) = self.nintendo_limited {
            if nintendo_limited {
                props.push("0".into());
            } else {
                props.push("1".into());
            }
        }

        if let Some(port) = &self.port {
            props.push(port.clone());
        }

        if let Some(ipv6_port) = &self.ipv6_port {
            props.push(ipv6_port.clone());
        }

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

        let edition = parts
            .get(0)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid motd edition",
            ))?
            .clone();

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

        let sub_name = parts
            .get(7)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid motd sub name",
            ))?
            .clone();

        let gamemode = parts
            .get(8)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid motd gamemode",
            ))?
            .clone();

        // now optional fields
        let nintendo_limited = parts.get(9).map(|c| c.clone());

        let port = parts.get(10).map(|c| c.clone());

        let ipv6_port = parts.get(11).map(|c| c.clone());

        Ok(Motd {
            edition,
            name,
            protocol: protocol.as_str().parse().unwrap(),
            version,
            player_count: player_count.parse().unwrap(),
            player_max: player_max.parse().unwrap(),
            server_guid: server_guid.parse().unwrap(),
            sub_name,
            gamemode: Gamemode::from_str(&gamemode).unwrap_or(Gamemode::Survival),
            nintendo_limited: nintendo_limited.map(|c| if c == "0" { true } else { false }),
            port,
            ipv6_port,
        })
    }
    // fn read(buf: &mut ByteReader) -> Result<Motd, std::io::Error> {
    //     let str_len = buf.read_u16()?;
    //     let mut str_buf = vec![0; str_len as usize];

    //     buf.read(&mut str_buf)?;

    //     let motd = String::from_utf8(str_buf).unwrap();

    //     let parts = motd
    //         .split(";")
    //         .map(|c| c.to_string())
    //         .collect::<Vec<String>>();

    //     let edition = parts
    //         .get(0)
    //         .ok_or(std::io::Error::new(
    //             std::io::ErrorKind::InvalidData,
    //             "Invalid motd edition",
    //         ))?
    //         .clone();

    //     let name = parts
    //         .get(1)
    //         .ok_or(std::io::Error::new(
    //             std::io::ErrorKind::InvalidData,
    //             "Invalid motd name",
    //         ))?
    //         .clone();

    //     let protocol = parts
    //         .get(2)
    //         .ok_or(std::io::Error::new(
    //             std::io::ErrorKind::InvalidData,
    //             "Invalid motd protocol",
    //         ))?
    //         .clone();

    //     let version = parts
    //         .get(3)
    //         .ok_or(std::io::Error::new(
    //             std::io::ErrorKind::InvalidData,
    //             "Invalid motd version",
    //         ))?
    //         .clone();

    //     let player_count = parts
    //         .get(4)
    //         .ok_or(std::io::Error::new(
    //             std::io::ErrorKind::InvalidData,
    //             "Invalid motd player count",
    //         ))?
    //         .clone();

    //     let player_max = parts
    //         .get(5)
    //         .ok_or(std::io::Error::new(
    //             std::io::ErrorKind::InvalidData,
    //             "Invalid motd player max",
    //         ))?
    //         .clone();

    //     let server_guid = parts
    //         .get(6)
    //         .ok_or(std::io::Error::new(
    //             std::io::ErrorKind::InvalidData,
    //             "Invalid motd server guid",
    //         ))?
    //         .clone();

    //     let sub_name = parts
    //         .get(7)
    //         .ok_or(std::io::Error::new(
    //             std::io::ErrorKind::InvalidData,
    //             "Invalid motd sub name",
    //         ))?
    //         .clone();

    //     let gamemode = parts
    //         .get(8)
    //         .ok_or(std::io::Error::new(
    //             std::io::ErrorKind::InvalidData,
    //             "Invalid motd gamemode",
    //         ))?
    //         .clone();

    //     let nintendo_limited = parts
    //         .get(9)
    //         .ok_or(std::io::Error::new(
    //             std::io::ErrorKind::InvalidData,
    //             "Invalid motd nintendo limited",
    //         ))?
    //         .clone();

    //     let port = parts
    //         .get(10)
    //         .ok_or(std::io::Error::new(
    //             std::io::ErrorKind::InvalidData,
    //             "Invalid motd port",
    //         ))?
    //         .clone();

    //     let ipv6_port = parts
    //         .get(11)
    //         .ok_or(std::io::Error::new(
    //             std::io::ErrorKind::InvalidData,
    //             "Invalid motd ipv6 port",
    //         ))?
    //         .clone();

    //     Ok(Motd {
    //         edition,
    //         name,
    //         protocol: protocol.as_str().parse().unwrap(),
    //         version,
    //         player_count: player_count.parse().unwrap(),
    //         player_max: player_max.parse().unwrap(),
    //         server_guid: server_guid.parse().unwrap(),
    //         sub_name,
    //         gamemode: Gamemode::from_str(&gamemode).unwrap_or(Gamemode::Survival),
    //         nintendo_limited: if nintendo_limited == "0" { true } else { false },
    //         port,
    //         ipv6_port,
    //     })
    // }
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
