use crate::conn::Connection;
use crate::Motd;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

pub enum RakNetVersion {
    MinecraftRecent,
    V10,
    V6,
}

impl RakNetVersion {
    pub fn to_u8(&self) -> u8 {
        match self {
            RakNetVersion::MinecraftRecent => 10,
            RakNetVersion::V10 => 10,
            RakNetVersion::V6 => 6,
        }
    }
}

#[derive(Clone, Debug)]
pub enum RakNetEvent {
    /// When a connection is created
    ///
    /// ! This is not the same as connecting to the server !
    ///
    /// **Tuple Values**:
    /// 1. The parsed `ip:port` address of the connection.
    ConnectionCreated(String),
    /// When a connection disconnects from the server
    /// Or the server forces the connection to disconnect
    ///
    /// **Tuple Values**:
    /// 1. The parsed `ip:port` address of the connection.
    /// 2. The reason for disconnect.
    Disconnect(String, String),
    /// When a connection is sent a motd.
    /// You should return a Motd here if you want to change the MOTD.
    ///
    /// **Tuple Values**:
    /// 1. The parsed `ip:port` address of the connection.
    /// 2. The `Motd` that might be sent.
    Motd(String, Motd),
    /// When a game packet is recieved.
    ///
    /// **Tuple Values**:
    /// 1. The parsed `ip:port` address of the connection.
    /// 2. The packet `Vec<u8>` recieved from the connection.
    GamePacket(String, Vec<u8>),
    /// When RakNet Errors in some way that is recoverable.
    ///
    /// **Tuple Values**:
    /// 1. The message to the error.
    Error(String),
    /// When a RakNet packet fails to parse or read a packet.
    /// While the reason can be anything, this is considered a level 2 error (almost critical)
    /// and should be handled by the server properly.
    ///
    /// **Tuple Values**:
    /// 1. The parsed `ip:port` of the connection that the packet was parsed for.
    /// 2. The packet `Vec<u8>` that was supposed to succeed.
    /// 3. The reason `String` for failing.
    ComplexBinaryError(String, Vec<u8>, String)
}

impl RakNetEvent {
    pub fn get_name(&self) -> String {
        match self.clone() {
            RakNetEvent::ConnectionCreated(_) => "ConnectionCreated".into(),
            RakNetEvent::Disconnect(_, _) => "Disconnect".into(),
            RakNetEvent::GamePacket(_, _) => "GamePacket".into(),
            RakNetEvent::Motd(_, _) => "Motd".into(),
            RakNetEvent::Error(_) => "Error".into(),
            RakNetEvent::ComplexBinaryError(_, _, _) => "ComplexBinaryError".into()
        }
    }
}

pub enum RakResult {
    /// Update the Motd for that specific client.
    ///
    /// **Tuple Values**:
    /// 1. The `Motd` for the current connection.
    Motd(Motd),
    /// Force the raknet server to invoke `panic!`.
    ///
    /// **Tuple Values**:
    /// 1. The message passed to `panic!`
    Error(String),
    /// Force the current client to disconnect.
    /// This does **NOT** emit a disonnect event.
    /// **Tuple Values**:
    /// 1. The reason for disconnect (if any).
    Disconnect(String),
}

pub type RakEventListenerFn = dyn FnMut(RakNetEvent) -> Option<RakResult> + Send + Sync;

pub struct RakNetServer {
    pub address: String,
    pub version: RakNetVersion,
    pub connections: Arc<Mutex<HashMap<String, Connection>>>,
    pub start_time: SystemTime,
    pub motd: Arc<Motd>,
}

impl RakNetServer {
    pub fn new(address: String) -> Self {
        Self {
            address,
            version: RakNetVersion::MinecraftRecent,
            connections: Arc::new(Mutex::new(HashMap::new())),
            start_time: SystemTime::now(),
            motd: Arc::new(Motd::default()),
        }
    }

    pub fn set_motd(&mut self, motd: Motd) {
        *Arc::get_mut(&mut self.motd).unwrap() = motd;
    }

    /// Sends a stream to the specified address.
    /// Instant skips the tick and forcefully sends the packet to the client.
    /// Params:
    /// - instant(true) -> Skips entire fragmentation and immediately sequences the
    ///   buffer into a stream.
    pub fn send(&mut self, address: String, stream: Vec<u8>, instant: bool) {
        let clients = self.connections.lock();
        match clients.unwrap().get_mut(&address) {
            Some(c) => c.send(stream, instant),
            None => return,
        };
    }
}
