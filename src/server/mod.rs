#[allow(unused)]
/// Server events module. Handles things like updating the MOTD
/// for certain connections. This is a notifier channel.
pub mod event;

use std::collections::HashMap;
use std::{net::SocketAddr, sync::Arc};

#[cfg(feature = "async_std")]
use async_std::{
    channel::{bounded, Receiver, Sender},
    net::UdpSocket,
    sync::Mutex,
    task::{self},
};
#[cfg(feature = "async_std")]
use futures::{select, FutureExt};

use binary_util::ByteReader;
use binary_util::interfaces::{Reader, Writer};

#[cfg(feature = "async_tokio")]
use tokio::{
    net::UdpSocket,
    sync::mpsc::channel as bounded,
    sync::mpsc::{Receiver, Sender},
    sync::Mutex,
    task::{self},
    select
};

use crate::connection::{ConnMeta, Connection};
use crate::error::server::ServerError;
use crate::notify::Notify;
use crate::protocol::mcpe::motd::Motd;
use crate::protocol::packet::offline::{
    IncompatibleProtocolVersion, OfflinePacket, OpenConnectReply, SessionInfoReply, UnconnectedPong,
};
use crate::protocol::packet::RakPacket;
use crate::protocol::Magic;
use crate::rakrs_debug;
use crate::util::to_address_token;

pub type Session = (ConnMeta, Sender<Vec<u8>>);

// stupid hack for easier syntax :)
pub enum PossiblySocketAddr<'a> {
    SocketAddr(SocketAddr),
    Str(&'a str),
    String(String),
    ActuallyNot,
}

impl PossiblySocketAddr<'_> {
    pub fn to_socket_addr(self) -> Option<SocketAddr> {
        match self {
            PossiblySocketAddr::SocketAddr(addr) => Some(addr),
            PossiblySocketAddr::Str(addr) => {
                // we need to parse it
                Some(addr.parse::<SocketAddr>().unwrap())
            }
            PossiblySocketAddr::String(addr) => {
                // same as above, except less elegant >_<
                Some(addr.clone().as_str().parse::<SocketAddr>().unwrap())
            }
            _ => None,
        }
    }
}

impl From<&str> for PossiblySocketAddr<'_> {
    fn from(s: &str) -> Self {
        PossiblySocketAddr::String(s.to_string())
    }
}

impl From<String> for PossiblySocketAddr<'_> {
    fn from(s: String) -> Self {
        PossiblySocketAddr::String(s)
    }
}

impl From<SocketAddr> for PossiblySocketAddr<'_> {
    fn from(s: SocketAddr) -> Self {
        PossiblySocketAddr::SocketAddr(s)
    }
}

impl std::fmt::Display for PossiblySocketAddr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PossiblySocketAddr::SocketAddr(addr) => write!(f, "{}", addr),
            PossiblySocketAddr::Str(addr) => write!(f, "{}", addr),
            PossiblySocketAddr::String(addr) => write!(f, "{}", addr),
            PossiblySocketAddr::ActuallyNot => write!(f, "Not a valid address!"),
        }
    }
}

pub struct Listener {
    /// If mcpe is true, this is the default MOTD, this is
    /// the default MOTD to send to the client. You can change this later by setting
    /// a motd in the `Conn` struct.
    pub motd: Motd,
    /// A server Id, passed in unconnected pong.
    pub id: u64,
    /// Supported versions
    pub versions: &'static [u8],
    /// Whether or not the server is being served.
    serving: bool,
    /// The current socket.
    sock: Option<Arc<UdpSocket>>,
    /// A Hashmap off all current connections along with a sending channel
    /// and some meta data like the time of connection, and the requested MTU_Size
    connections: Arc<Mutex<HashMap<SocketAddr, Session>>>,
    /// The recieve communication channel, This is used to dispatch connections between a handle
    /// It allows you to use the syntax sugar for `Listener::accept()`.
    recv_comm: Receiver<Connection>,
    send_comm: Sender<Connection>,
    // TODO, fix this!
    // send_evnt: Sender<(ServerEvent, oneshot::Sender<ServerEventResponse>)>,
    // pub recv_evnt: Arc<Mutex<mpsc::Receiver<(ServerEvent, oneshot::Sender<ServerEventResponse>)>>>,
    // TODO
    /// A Notifier (sephamore) that will wait until all notified listeners
    /// are completed, and finish closing.
    closed: Arc<Notify>,
    // This is a notifier that acknowledges all connections have been removed from the server successfully.
    // This is important to prevent memory leaks if the process is continously running.
    // cleanup: Arc<Condvar>,
}

impl Listener {
    /// Binds a socket to the specified addres and starts listening.
    pub async fn bind<I: for<'a> Into<PossiblySocketAddr<'a>>>(
        address: I,
    ) -> Result<Self, ServerError> {
        let a: PossiblySocketAddr = address.into();
        let address_r: Option<SocketAddr> = a.to_socket_addr();
        if address_r.is_none() {
            rakrs_debug!("Invalid binding value");
            return Err(ServerError::AddrBindErr);
        }

        let address = address_r.unwrap();

        let sock = match UdpSocket::bind(address).await {
            Ok(s) => s,
            Err(_) => return Err(ServerError::AddrBindErr),
        };

        let server_id: u64 = rand::random();
        let motd = Motd::new(server_id, format!("{}", address.port()));

        // This channel is a Communication channel for when `Connection` structs are initialized.
        let (send_comm, recv_comm) = bounded::<Connection>(10);
        // This channel is responsible for handling and dispatching events between clients.
        // Oneshot will garauntee this event is intended for the client whom requested the event.
        // TODO: Fix with new event system
        // let (send_evnt, recv_evnt) =
        //     mpsc::channel::<(ServerEvent, oneshot::Sender<ServerEventResponse>)>(10);

        let listener = Self {
            sock: Some(Arc::new(sock)),
            id: server_id,
            versions: &[10, 11],
            motd,
            send_comm,
            recv_comm,
            // send_evnt,
            // recv_evnt: Arc::new(Mutex::new(recv_evnt)),
            serving: false,
            connections: Arc::new(Mutex::new(HashMap::new())),
            // closer: Arc::new(Semaphore::new(0)),
            closed: Arc::new(Notify::new()),
            // cleanup: Arc::new(Notify::new()),
            // cleanup: Arc::new(Condvar::new()),
        };

        return Ok(listener);
    }

    /// Starts the listener!
    /// TODO
    /// ```ignore
    /// use rak_rs::server::Listener;
    /// async fn start() {
    ///     let mut server = Listener::bind("0.0.0.0:19132").await.unwrap();
    ///
    ///     // let's begin to listen to connections
    ///     server.start().await;
    /// }
    /// ```
    pub async fn start(&mut self) -> Result<(), ServerError> {
        if self.serving {
            return Err(ServerError::AlreadyOnline);
        }

        let socket = self.sock.as_ref().unwrap().clone();
        let send_comm = self.send_comm.clone();
        // let send_evt = self.send_evnt.clone();
        let server_id = self.id.clone();
        #[cfg(feature = "mcpe")]
        let default_motd = self.motd.clone();
        let connections = self.connections.clone();
        let closer = self.closed.clone();
        let connections2 = self.connections.clone();
        let closer2 = self.closed.clone();
        let versions = self.versions.clone();

        self.serving = true;

        #[cfg(feature = "async_std")]
        let (cs, client_close_recv) = bounded::<SocketAddr>(10);
        #[cfg(feature = "async_tokio")]
        let (cs, mut client_close_recv) = bounded::<SocketAddr>(10);
        let client_close_send = Arc::new(cs);

        task::spawn(async move {
            // We allocate here to prevent constant allocation of this array
            let mut buf: [u8; 2048] = [0; 2048];
            #[cfg(feature = "mcpe")]
            let motd_default = default_motd.clone();

            loop {
                let length: usize;
                let origin: SocketAddr;

                macro_rules! recv_body {
                    ($recv: ident) => {
                        match $recv {
                            Ok((l, o)) => {
                                length = l;
                                origin = o;
                            }
                            Err(e) => {
                                rakrs_debug!(true, "Error: {:?}", e);
                                continue;
                            }
                        }

                        // Do a quick check to see if this a valid raknet packet, otherwise we're going to handle it normally
                        if let Ok(pk) = OfflinePacket::read(&mut ByteReader::from(&buf[..length])) {
                            // Offline packets are not buffered to the user.
                            // The reason for this is because we don't wish for the user to be able to disrupt
                            // raknet protocol, and handshaking.
                            match pk {
                                OfflinePacket::UnconnectedPing(_) => {
                                    // let (resp_tx, resp_rx) =
                                    //     oneshot::channel::<ServerEventResponse>();
                                    #[cfg(feature = "mcpe")]
                                    let motd: Motd = motd_default.clone();

                                    // if let Err(e) = send_evt.try_send((
                                    //         ServerEvent::RefreshMotdRequest(origin, motd.clone()),
                                    //         // resp_tx,
                                    //     ))
                                    // {
                                    //     match e {
                                    //         TrySendError::Full(_) => {
                                    //             rakrs_debug!(true, "[{}] Event dispatcher is full! Dropping request.", to_address_token(origin));
                                    //         }
                                    //         TrySendError::Closed(_) => {
                                    //             rakrs_debug!(true, "[{}] Event dispatcher is closed! Dropping request.", to_address_token(origin));
                                    //         }
                                    //     }
                                    // }

                                    // if let Ok(res) = resp_rx.await {
                                    //     // get the motd from the server event otherwise use defaults.
                                    //     // if let Ok(res) = res {
                                    //         match res {
                                    //             ServerEventResponse::RefreshMotd(m) => {
                                    //                 motd = m;
                                    //             }
                                    //             _ => {
                                    //                 rakrs_debug!(true, "[{}] Response to ServerEvent::RefreshMotdRequest is invalid!", to_address_token(origin));
                                    //             }
                                    //         }
                                    //     // };
                                    // }

                                    // unconnected pong signature is different if MCPE is specified.
                                    let resp = UnconnectedPong {
                                        timestamp: current_epoch(),
                                        server_id,
                                        magic: Magic::new(),
                                        #[cfg(feature = "mcpe")]
                                        motd,
                                    };

                                    send_packet_to_socket(&socket, resp.into(), origin).await;
                                    continue;
                                }
                                OfflinePacket::OpenConnectRequest(mut pk) => {
                                    // TODO make a constant for this
                                    if !versions.contains(&pk.protocol) {
                                        let resp = IncompatibleProtocolVersion {
                                            protocol: pk.protocol,
                                            magic: Magic::new(),
                                            server_id,
                                        };

                                        rakrs_debug!("[{}] Sent ({}) which is invalid RakNet protocol. Version is incompatible with server.", pk.protocol, to_address_token(*&origin));

                                        send_packet_to_socket(&socket, resp.into(), origin).await;
                                        continue;
                                    }

                                    rakrs_debug!(
                                        true,
                                        "[{}] Client requested Mtu Size: {}",
                                        to_address_token(*&origin),
                                        pk.mtu_size
                                    );

                                    if pk.mtu_size > 2048 {
                                        rakrs_debug!(
                                            true,
                                            "[{}] Client requested Mtu Size: {} which is larger than the maximum allowed size of 2048",
                                            to_address_token(*&origin),
                                            pk.mtu_size
                                        );
                                        pk.mtu_size = 2048;
                                    }

                                    let resp = OpenConnectReply {
                                        server_id,
                                        // TODO allow encryption
                                        security: false,
                                        magic: Magic::new(),
                                        // TODO make this configurable, this is sent to the client to change
                                        // it's mtu size, right now we're using what the client prefers.
                                        // however in some cases this may not be the preferred use case, for instance
                                        // on servers with larger worlds, you may want a larger mtu size, or if
                                        // your limited on network bandwith
                                        mtu_size: pk.mtu_size,
                                    };
                                    send_packet_to_socket(&socket, resp.into(), origin).await;
                                    continue;
                                }
                                OfflinePacket::SessionInfoRequest(pk) => {
                                    let resp = SessionInfoReply {
                                        server_id,
                                        client_address: origin,
                                        magic: Magic::new(),
                                        mtu_size: pk.mtu_size,
                                        security: false,
                                    };

                                    // This is a valid packet, let's check if a session exists, if not, we should create it.
                                    // Event if the connection is only in offline mode.
                                    let mut sessions = connections.lock().await;

                                    if !sessions.contains_key(&origin) {
                                        rakrs_debug!(true, "Creating new session for {}", origin);
                                        let meta = ConnMeta::new(0);
                                        let (net_send, net_recv) = bounded::<Vec<u8>>(10);
                                        let connection =
                                            Connection::new(origin, &socket, net_recv, client_close_send.clone(), pk.mtu_size).await;
                                        rakrs_debug!(true, "Created Session for {}", origin);

                                        // Add the connection to the available connections list.
                                        // we're using the name "sessions" here to differeniate
                                        // for some reason the reciever likes to be dropped, so we're saving it here.
                                        sessions.insert(origin, (meta, net_send));

                                        // notify the connection communicator
                                        if let Err(err) = send_comm.send(connection).await {
                                            let connection = err.0;
                                            // there was an error, and we should terminate this connection immediately.
                                            rakrs_debug!("[{}] Error while communicating with internal connection channel! Connection withdrawn.", to_address_token(connection.address));
                                            sessions.remove(&origin);
                                            continue;
                                        }
                                    }

                                    // update the sessions mtuSize, this is referred to internally, we also will send this event to the client
                                    // event channel. However we are not expecting a response.

                                    sessions.get_mut(&origin).unwrap().0.mtu_size = pk.mtu_size;
                                    rakrs_debug!(
                                        true,
                                        "[{}] Updated mtu size to {}",
                                        to_address_token(origin),
                                        pk.mtu_size
                                    );

                                    // let (resp_tx, resp_rx) = oneshot::channel::<ServerEventResponse>();

                                    // if let Err(_) = timeout(Duration::from_millis(5), resp_rx).await {
                                    //     rakrs_debug!(
                                    //         "[{}] Failed to update mtu size with the client!",
                                    //         to_address_token(origin)
                                    //     );
                                    // }

                                    // if let Err(_) = send_evt.send((ServerEvent::SetMtuSize(pk.mtu_size), resp_tx))
                                    //     .await
                                    // {
                                    //     rakrs_debug!(
                                    //         "[{}] Failed to update mtu size with the client!",
                                    //         to_address_token(origin)
                                    //     );
                                    // }

                                    send_packet_to_socket(&socket, resp.into(), origin).await;
                                    continue;
                                }
                                _ => {
                                    rakrs_debug!(
                                        "[{}] Received invalid packet!",
                                        to_address_token(*&origin)
                                    );
                                }
                            }
                        }

                        // Packet may be valid, but we'll let the connection decide this
                        let mut sessions = connections.lock().await;
                        if sessions.contains_key(&origin) {
                            if let Err(_) = sessions[&origin].1.send(buf[..length].to_vec()).await {
                                rakrs_debug!(true, "[{}] Failed when handling recieved packet! Could not pass over to internal connection, the channel might be closed! (Removed the connection)", to_address_token(*&origin));
                                sessions.remove(&origin);
                            }
                        }
                        drop(sessions);
                    };
                }

                #[cfg(feature = "async_std")]
                select! {
                    _ = closer.wait().fuse() => {
                        rakrs_debug!(true, "[SERVER] [NETWORK] Server has recieved the shutdown notification!");
                        break;
                    }
                    recv = socket.recv_from(&mut buf).fuse() => {
                       recv_body!(recv);
                    }
                }

                #[cfg(feature = "async_tokio")]
                select! {
                    _ = closer.wait() => {
                        rakrs_debug!(true, "[SERVER] [NETWORK] Server has recieved the shutdown notification!");
                        break;
                    }
                    recv = socket.recv_from(&mut buf) => {
                        recv_body!(recv);
                    }
                }
            }
        });

        task::spawn(async move {
            // here we loop and recv from the client_close_recv channel
            // and remove the connection from the hashmap
            loop {
                #[cfg(feature = "async_std")]
                select! {
                    _ = closer2.wait().fuse() => {
                        rakrs_debug!(true, "[SERVER] [Cleanup] Server has recieved the shutdown notification!");
                        break;
                    }
                    addr = client_close_recv.recv().fuse() => {
                        if let Ok(addr) = addr {
                            rakrs_debug!(true, "[SERVER] [Cleanup] Removing connection for {}", to_address_token(addr));
                            connections2.lock().await.remove(&addr);
                        }
                    }
                }

                #[cfg(feature = "async_tokio")]
                select! {
                    _ = closer2.wait() => {
                        rakrs_debug!(true, "[SERVER] [Cleanup] Server has recieved the shutdown notification!");
                        break;
                    }
                    addr = client_close_recv.recv() => {
                        if let Some(addr) = addr {
                            rakrs_debug!(true, "[SERVER] [Cleanup] Removing connection for {}", to_address_token(addr));
                            connections2.lock().await.remove(&addr);
                        }
                    }
                }
            }
        });

        return Ok(());
    }

    // pub async fn recv_event(&self) -> Result<(ServerEvent, oneshot::Sender<ServerEventResponse>), ServerError> {
    //     if !self.serving {
    //         Err(ServerError::NotListening)
    //     } else {
    //         let mut recvr = self.recv_evnt.lock().await;
    //         tokio::select! {
    //             receiver = recvr.recv() => {
    //                 match receiver {
    //                     Some(c) => Ok(c),
    //                     None => Err(ServerError::Killed)
    //                 }
    //             },
    //             _ = self.closer.acquire() => {
    //                 Err(ServerError::Killed)
    //             }
    //         }
    //     }
    // }

    /// Must be called in after both `Listener::bind` AND `Listener::start`. This function
    /// is used to recieve and accept connections. Alternatively, you can refuse a connection
    /// by dropping it when you accept it.
    pub async fn accept(&mut self) -> Result<Connection, ServerError> {
        if !self.serving {
            Err(ServerError::NotListening)
        } else {
            let receiver = self.recv_comm.recv().await;
            return match receiver {
                #[cfg(feature = "async_std")]
                Ok(c) => Ok(c),
                #[cfg(feature = "async_std")]
                Err(_) => Err(ServerError::Killed),
                #[cfg(feature = "async_tokio")]
                Some(c) => Ok(c),
                #[cfg(feature = "async_tokio")]
                None => Err(ServerError::Killed),
            };
        }
    }

    /// Stops the listener and frees the socket.
    pub async fn stop(&mut self) -> Result<(), ServerError> {
        self.closed.notify().await;
        // self.cleanup.notified().await;

        self.sock = None;
        self.serving = false;

        Ok(())
    }
}

async fn send_packet_to_socket(socket: &Arc<UdpSocket>, packet: RakPacket, origin: SocketAddr) {
    if let Err(e) = socket
        .send_to(&mut packet.write_to_bytes().unwrap().as_slice(), origin)
        .await
    {
        rakrs_debug!(
            "[{}] Failed sending payload to socket! {}",
            to_address_token(origin),
            e
        );
    }
}

pub(crate) fn current_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as u64
}
