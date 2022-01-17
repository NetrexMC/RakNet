use binary_utils::Streamable;

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
    pub player_count: u16,
    /// The maximum number of players
    pub player_max: u16,
    /// The gamemode of the server
    pub gamemode: Gamemode,
    /// The server's GUID
    pub server_guid: u64,
    /// The server's port
    pub port: String,
    /// The IPv6 port
    /// TODO: Implement this
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
            self.gamemode.to_string(),
            self.port.to_string(),
            self.ipv6_port.to_string(),
        ];
        props.join(";").into()
    }
}

impl Streamable for Motd {
    fn compose(
        source: &[u8],
        position: &mut usize,
    ) -> Result<Self, binary_utils::error::BinaryError> {
        let motd = String::compose(source, position)?;
        let parts = motd
            .split(";")
            .map(|c| c.to_string())
            .collect::<Vec<String>>();
        let name = parts
            .get(1)
            .ok_or(binary_utils::error::BinaryError::RecoverableKnown(
                "Invalid motd name".into(),
            ))?;
        let protocol = parts
            .get(2)
            .ok_or(binary_utils::error::BinaryError::RecoverableKnown(
                "Invalid motd protocol".into(),
            ))?;
        let version = parts
            .get(3)
            .ok_or(binary_utils::error::BinaryError::RecoverableKnown(
                "Invalid motd version".into(),
            ))?;
        let player_count =
            parts
                .get(4)
                .ok_or(binary_utils::error::BinaryError::RecoverableKnown(
                    "Invalid motd player count".into(),
                ))?;
        let player_max = parts
            .get(5)
            .ok_or(binary_utils::error::BinaryError::RecoverableKnown(
                "Invalid motd player maxmium".into(),
            ))?;
        let server_guid =
            parts
                .get(6)
                .ok_or(binary_utils::error::BinaryError::RecoverableKnown(
                    "Invalid motd server guid".into(),
                ))?;
        let _ = parts
            .get(7)
            .ok_or(binary_utils::error::BinaryError::RecoverableKnown(
                "Invalid motd software name".into(),
            ))?;
        let _ = parts
            .get(8)
            .ok_or(binary_utils::error::BinaryError::RecoverableKnown(
                "Invalid motd gamemode string".into(),
            ))?;
        let gamemode = parts
            .get(9)
            .ok_or(binary_utils::error::BinaryError::RecoverableKnown(
                "Invalid motd gamemode".into(),
            ))?;
        let port = parts
            .get(10)
            .ok_or(binary_utils::error::BinaryError::RecoverableKnown(
                "Invalid motd port".into(),
            ))?;
        let ipv6_port = parts
            .get(11)
            .ok_or(binary_utils::error::BinaryError::RecoverableKnown(
                "Invalid motd port".into(),
            ))?;

        Ok(Motd {
            name: name.clone(),
            protocol: protocol
                .as_str()
                .parse::<u16>()
                .expect("Invalid motd protocol"),
            version: version.clone(),
            player_count: player_count
                .as_str()
                .parse::<u16>()
                .expect("Player count is not a number"),
            player_max: player_max
                .as_str()
                .parse::<u16>()
                .expect("Player Maximum is not a number"),
            server_guid: server_guid
                .as_str()
                .parse::<u64>()
                .expect("Server GUID is not a number"),
            port: port.clone(),
            ipv6_port: ipv6_port.clone(),
            gamemode: match gamemode
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
        })
    }

    fn parse(&self) -> Result<Vec<u8>, binary_utils::error::BinaryError> {
        self.write().parse()
    }
}