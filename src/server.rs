#![feature(in_band_lifetimes)]
use netrex_events::Channel;
use tokio::net::UdpSocket;
use tokio::time::sleep;

use crate::conn::{Connection, ConnectionState};
use crate::{Motd, from_tokenized};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, Duration};

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
pub enum RakEvent {
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
    ComplexBinaryError(String, Vec<u8>, String),
}

impl RakEvent {
    pub fn get_name(&self) -> String {
        match self.clone() {
            RakEvent::ConnectionCreated(_) => "ConnectionCreated".into(),
            RakEvent::Disconnect(_, _) => "Disconnect".into(),
            RakEvent::GamePacket(_, _) => "GamePacket".into(),
            RakEvent::Motd(_, _) => "Motd".into(),
            RakEvent::Error(_) => "Error".into(),
            RakEvent::ComplexBinaryError(_, _, _) => "ComplexBinaryError".into(),
        }
    }
}

#[derive(Clone, Debug)]
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

pub struct RakNetServer {
    pub address: String,
    pub version: RakNetVersion,
    pub connections: Mutex<HashMap<String, Connection>>,
    pub start_time: SystemTime,
    pub motd: Motd,
    stop: bool,
}

impl RakNetServer {
    pub fn new(address: String) -> Self {
        Self {
            address,
            version: RakNetVersion::MinecraftRecent,
            connections: Mutex::new(HashMap::new()),
            start_time: SystemTime::now(),
            motd: Motd::default(),
            stop: false
        }
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

    pub async fn start<'a>(&mut self, send_channel: Channel<'a, RakEvent, RakResult>) {
        let socket = UdpSocket::bind(self.address.parse::<SocketAddr>().expect("Failed to bind to address.")).await.unwrap();

        async {
            while !&self.stop {
                if let Err(_) = socket.readable().await {
                    continue;
                };

                let mut buf = [0; 2048];
                if let Ok((len, addr)) = socket.recv_from(&mut buf).await {
                    let data = &buf[..len];

                    println!("[RakNet] {}: Sent packet: {:?}", addr, &data);

                    if let Ok(mut clients) = self.connections.lock() {
                        if let Some(c) = clients.get_mut(&addr.ip().to_string()) {
                            c.recv(&data.to_vec());
                        } else {
                            // add the client!
                            // we need to add cooldown here eventually.
                            if !self.connections.lock().unwrap().contains_key(&addr.ip().to_string()) {
                                let mut c = Connection::new(addr, self.start_time, self.motd.clone());
                                c.recv(&data.to_vec());
                                clients.insert(addr.ip().to_string(), c);
                            } else {
                                // throw an error, this should never happen.
                                println!("[RakNet] Failed to add client, address already exists but failed mutable borrow.");
                            }
                        }
                    }
                } else {
                    // log error in future!
                    println!("[RakNet] Unknown error decoding packet!");
                    continue;
                }
            }
        }.await;

        async {
            while !&self.stop {
                if let Err(_) = socket.writable().await {
                    continue;
                };

                // sleep an entire tick
                sleep(Duration::from_millis(50)).await;

                let mut clients = self.connections.lock().unwrap();
                for (addr, _) in clients.clone().iter() {
                    let client = clients.get_mut(addr).expect("Could not get connection");
                    client.do_tick();

                    let dispatch = client.event_dispatch.clone();
                    client.event_dispatch.clear();

                    // emit events if there is a listener for the
                    for event in dispatch.iter() {
                        // println!("DEBUG => Dispatching: {:?}", &event.get_name());
                        if let Some(result) = send_channel.send(event.clone()) {
                            match result {
                                RakResult::Motd(_v) => {
                                    // we don't really support changing
                                    // client MOTD at the moment...
                                    // so we don't do anything for this.
                                }
                                RakResult::Error(v) => {
                                    // Calling error forces an error to raise.
                                    panic!("{}", v);
                                }
                                RakResult::Disconnect(_) => {
                                    client.state = ConnectionState::Offline; // simple hack
                                    break;
                                }
                            }
                        }
                    }

                    if client.state == ConnectionState::Offline {
                        clients.remove(addr);
                        continue;
                    }

                    if client.send_queue.len() == 0 {
                        continue;
                    }

                    for pk in client.clone().send_queue.into_iter() {
                        match socket.send_to(&pk[..], &from_tokenized(addr.clone())).await
                        {
                            // Add proper handling!
                            Err(e) => eprintln!("Error Sending Packet [{}]: ", e),
                            Ok(_) => continue, // println!("\nSent Packet [{}]: {:?}", addr, pk)
                        }
                    }
                    client.send_queue.clear();
                }
                drop(clients);
            }
        }.await;
    }
}
