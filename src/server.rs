use futures::Future;
use netrex_events::Channel;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;
use std::time::SystemTime;
use tokio::net::UdpSocket;
use tokio::time::sleep;

use crate::connection::state::ConnectionState;
use crate::connection::Connection;
use crate::internal::util::from_address_token;
use crate::internal::util::to_address_token;
use crate::protocol::mcpe::motd::Motd;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum RakNetVersion {
    V10 = 10,
    V6 = 6,
}

impl RakNetVersion {
    pub fn to_u8(&self) -> u8 {
        match self {
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
        match self {
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
    pub connections: Arc<RwLock<HashMap<String, Connection>>>,
    pub start_time: SystemTime,
    pub server_guid: u64,
    pub stop: bool,
}

impl RakNetServer {
    pub fn new(address: String) -> Self {
        Self {
            address,
            version: RakNetVersion::V10,
            connections: Arc::new(RwLock::new(HashMap::new())),
            start_time: SystemTime::now(),
            server_guid: rand::random::<u64>(),
            stop: false,
        }
    }
}

pub async fn start<'a>(
    s: RakNetServer,
    send_channel: Channel<'a, RakEvent, RakResult>,
) -> (
    impl Future + 'a,
    Arc<RakNetServer>,
    tokio::sync::mpsc::Sender<(String, Vec<u8>, bool)>,
) {
    // The actual server reference.
    let server = Arc::new(s);
    // The reference to the server for the sending thread.
    // This thread is responsible for ticking the client and
    // dispatching client events every tick.
    let send_server = server.clone();
    // The reference to the server for the for sending packets.
    // This is the task that is used to send packets to the clients
    // from the api. This mspc channel is return to the user.
    let task_server = send_server.clone();
    // The reference to the server for returning the raknet server.
    // While the sender should already have this, the server does become
    // owned and pushed out of scope after execution.
    let ret_server = send_server.clone();
    let sock = UdpSocket::bind(
        server
            .address
            .parse::<SocketAddr>()
            .expect("Failed to bind to address."),
    )
    .await
    .unwrap();
    let port = server.address.parse::<SocketAddr>().unwrap().port();
    // The socket of the server for sending packets (ticking client thread).
    let send_sock = Arc::new(sock);
    // The socket for the recieving thread.
    let socket = send_sock.clone();
    // The socket for the internal server sending thread.
    let send_sock_internal = send_sock.clone();
    // The time we're going to say raknet actually started.
    let start_time = server.start_time.clone();
    // The id of the server
    let server_id = server.server_guid.clone();
    // The server of the server
    let version = server.version.clone();
    // The channels being used to send packets to the client (externally).
    let (send, mut recv) = tokio::sync::mpsc::channel::<(String, Vec<u8>, bool)>(2048);
    // The internal channels being used to dispatch packets with `connection.send`.
    let (im_send, mut im_recv) = tokio::sync::mpsc::channel::<(String, Vec<u8>)>(2048);

    let tasks = async move {
        // This task is solely responsible for internal immediate sending.
        // Nothing else, this is not used externally, nor should it be.
        tokio::spawn(async move {
            loop {
                if let Some(data) = im_recv.recv().await {
                    if let Ok(_) = send_sock_internal
                        .send_to(&data.1, from_address_token(data.0))
                        .await
                    {
                        continue;
                    } else {
                        println!("Failed to send immediate packet.");
                    }
                }
            }
        });

        tokio::spawn(async move {
            loop {
                if let Some((address, buf, instant)) = recv.recv().await {
                    let mut clients = task_server.connections.write().unwrap();
                    if clients.contains_key(&address) {
                        let client = clients.get_mut(&address).unwrap();
                        client.send(buf, instant);
                        drop(client);
                        drop(clients);
                    } else {
                        drop(clients);
                    }
                }
            }
        });

        tokio::spawn(async move {
            let internal_send = Arc::new(im_send);
            loop {
                if let Err(_) = socket.readable().await {
                    continue;
                };

                let mut buf = [0; 2048];
                if let Ok((len, addr)) = socket.recv_from(&mut buf).await {
                    let data = &buf[..len];
                    let address_token = to_address_token(addr);

                    // // println!("[RakNet] [{}] Received packet: Packet(ID={:#04x})", addr, &data[0]);

                    if let Ok(mut clients) = server.connections.write() {
                        if let Some(c) = clients.get_mut(&address_token) {
                            c.recv(&data.to_vec());
                        } else {
                            // add the client!
                            // we need to add cooldown here eventually.
                            if !clients.contains_key(&address_token) {
                                let mut c = Connection::new(
                                    address_token.clone(),
                                    internal_send.clone(),
                                    start_time,
                                    server_id,
                                    port.to_string(),
                                    version.clone(),
                                );
                                c.recv(&data.to_vec());
                                clients.insert(address_token, c);
                            } else {
                                // throw an error, this should never happen.
                            }
                        }
                    }
                } else {
                    // log error in future!
                    // println!("[RakNet] Unknown error decoding packet!");
                    continue;
                }
            }
        });

        while !&send_server.stop {
            if let Err(_) = send_sock.writable().await {
                continue;
            };

            // sleep an entire tick
            sleep(Duration::from_millis(50)).await;

            let mut clients = send_server.connections.write().unwrap();
            for (addr, _) in clients.clone().iter() {
                let client = clients.get_mut(addr).expect("Could not get connection");
                client.tick();

                let dispatch = client.event_dispatch.clone();
                client.event_dispatch.clear();

                let mut force_disconnect = false;

                // emit events if there is a listener for the
                for event in dispatch.iter() {
                    // // println!("DEBUG => Dispatching: {:?}", &event.get_name());
                    if let Some(result) = send_channel.send(event.clone()) {
                        match result {
                            RakResult::Motd(v) => {
                                client.motd = v;
                            }
                            RakResult::Error(v) => {
                                // Calling error forces an error to raise.
                                panic!("{}", v);
                            }
                            RakResult::Disconnect(_) => {
                                client.state = ConnectionState::Offline; // simple hack
                                force_disconnect = true;
                                break;
                            }
                        }
                    }
                }

                if client.state == ConnectionState::Offline || force_disconnect {
                    clients.remove(addr);
                    continue;
                }

                if client.queue.clone().len() == 0 {
                    continue;
                }

                let packets = client.queue.flush();

                for pk in packets.into_iter() {
                    match send_sock
                        .send_to(&pk[..], &from_address_token(addr.clone()))
                        .await
                    {
                        // Add proper handling!
                        Err(e) => println!("[RakNet] [{}] Error sending packet: {}", addr, e),
                        Ok(_) => {
                            if client.state.is_connected() {
                                if cfg!(any(test, feature = "dbg-verbose")) {
                                    println!("[ONLINE PACKET] [{}] Sent packet: {:?}\n", addr, &pk);
                                } else {
                                    println!(
                                        "[ONLINE PACKET] [{}] Sent packet: {}",
                                        addr,
                                        *pk.get(0).unwrap_or(&0)
                                    );
                                }
                            } else {
                                println!(
                                    "[OFFLINE] [{}] Sent packet: {}",
                                    addr,
                                    *pk.get(0).unwrap_or(&0)
                                );
                            }
                        }
                    }
                }
            }
            drop(clients);
        }
    };

    return (tasks, ret_server, send);
}
