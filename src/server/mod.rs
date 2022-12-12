/// Server events module. Handles things like updating the MOTD
/// for certain connections. This is a notifier channel.
pub mod event;

use std::collections::HashMap;
use std::{net::SocketAddr, sync::Arc};

use binary_utils::Streamable;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tokio::sync::{mpsc, oneshot};

use crate::conn::{Conn, ConnMeta};
use crate::error::server::ServerError;
use crate::error::{self, server};
use crate::protocol::mcpe::motd::Motd;
use crate::protocol::packet::offline::{
    IncompatibleProtocolVersion, OfflinePacket, OpenConnectReply, SessionInfoReply, UnconnectedPong,
};
use crate::protocol::packet::{Packet, Payload};
use crate::protocol::Magic;
use crate::server::event::ServerEventResponse;

use self::event::ServerEvent;

pub type Connection = (
    ConnMeta,
    mpsc::Sender<Vec<u8>>,
    mpsc::Sender<(ServerEvent, oneshot::Sender<ServerEventResponse>)>,
);
pub struct Listener {
    /// If mcpe is true, this is the default MOTD, this is
    /// the default MOTD to send to the client. You can change this later by setting
    /// a motd in the `Conn` struct.
    pub motd: Motd,
    /// A server Id, passed in unconnected pong.
    pub id: u64,
    /// Whether or not the server is being served.
    serving: bool,
    /// The current socket.
    sock: Option<Arc<UdpSocket>>,
    /// A Hashmap off all current connections along with a sending channel
    /// and some meta data like the time of connection, and the requested MTU_Size
    connections: Arc<Mutex<HashMap<SocketAddr, Connection>>>,

    /// The recieve communication channel, This is used to dispatch connections between a handle
    /// It allows you to use the syntax sugar for `Listener::accept()`.
    recv_comm: mpsc::Receiver<Conn>,
    send_comm: mpsc::Sender<Conn>,
}

impl Listener {
    /// Binds a socket to the specified addres and starts listening.
    /// EG:
    /// ```rust
    /// let mut server = Listener::bind("0.0.0.0:19132").await;
    /// let (conn, stream) = server.accept().await?.unwrap();
    /// ```
    pub async fn bind(address: SocketAddr) -> Result<Self, ServerError> {
        let sock = match UdpSocket::bind(address).await {
            Ok(s) => s,
            Err(_) => return Err(ServerError::AddrBindErr),
        };

        let server_id: u64 = rand::random();
        let motd = Motd::new(server_id, format!("{}", address.port()));

        // The buffer is 100 here because locking on large servers may cause
        // locking to take longer, and this accounts for that.
        let (send_comm, recv_comm) = mpsc::channel::<Conn>(100);
        let (send_evnt, recv_evnt) =
            mpsc::channel::<(ServerEvent, oneshot::Sender<ServerEventResponse>)>(10);

        let listener = Self {
            sock: Some(Arc::new(sock)),
            id: server_id,
            motd,
            send_comm,
            recv_comm,
            serving: false,
            connections: Arc::new(Mutex::new(HashMap::new())),
        };

        return Ok(listener);
    }

    /// Starts the listener!
    /// ```rust
    /// let mut server = Listener::bind("0.0.0.0:19132", true).await;
    ///
    /// // let's begin to listen to connections
    /// server.start().await;
    /// ```
    pub async fn start(&mut self) -> Result<(), ServerError> {
        if self.serving {
            return Err(ServerError::ServerRunning);
        }

        let socket = self.sock.as_ref().unwrap().clone();
        let send_comm = self.send_comm.clone();
        let server_id = self.id.clone();
        let default_motd = self.motd.clone();
        let connections = self.connections.clone();

        tokio::spawn(async move {
            // We allocate here to prevent constant allocation of this array
            let mut buf: [u8; 2048] = [0; 2048];
            let motd_default = default_motd.clone();

            loop {
                // The socket is not readable. We can not read it.
                if socket.readable().await.is_err() {
                    continue;
                }

                let length: usize;
                let origin: SocketAddr;

                // We need to wait for either the socket to stop recieving,
                // or a server close notification.
                tokio::select! {
                    recv = socket.recv_from(&mut buf) => {
                        match recv {
                            Ok((l, o)) => {
                                length = l;
                                origin = o;
                            },
                            Err(_) => continue
                        }
                    },
                    // todo disconnect notification
                };

                // Do a quick check to see if this a valid raknet packet, otherwise we're going to handle it normally
                if let Ok(packet) = Packet::compose(&mut buf, &mut 0) {
                    // This is a valid packet, let's check if a session exists, if not, we should create it.
                    // Event if the connection is only in offline mode.
                    let mut sessions = connections.lock().await;

                    if !sessions.contains_key(&origin) {
                        let meta = ConnMeta::new(0);
                        let (net_send, net_recv) = mpsc::channel::<Vec<u8>>(10);
                        let (evt_send, evt_recv) = mpsc::channel::<(
                            ServerEvent,
                            oneshot::Sender<ServerEventResponse>,
                        )>(10);
                        let connection = Conn::new(origin, &socket, net_recv, evt_recv);

                        // Add the connection to the available connections list.
                        // we're using the name "sessions" here to differeniate
                        sessions.insert(origin, (meta, net_send, evt_send));

                        // notify the connection communicator
                        if let Err(err) = send_comm.send(connection).await {
                            // there was an error, and we should terminate this connection immediately.
                            sessions.remove(&origin);
                            continue;
                        }
                    }

                    // We're dropping here because we don't know if we'll need it
                    // We don't want to hold the lock longer than we need to.
                    drop(sessions);

                    // if this is an offline packet, we can retrieve the buffer we should send.
                    match packet.payload {
                        Payload::Offline(pk) => {
                            // Offline packets are not buffered to the user.
                            // The reason for this is because we don't wish for the user to be able to disrupt
                            // raknet protocol, and handshaking.
                            match pk {
                                OfflinePacket::UnconnectedPing(_) => {
                                    let (resp_tx, resp_rx) =
                                        oneshot::channel::<ServerEventResponse>();
                                    let mut motd: Motd = motd_default.clone();
                                    let sessions = connections.lock().await;

                                    if let Err(err) = sessions[&origin]
                                        .2
                                        .send((
                                            ServerEvent::RefreshMotdRequest(origin, motd.clone()),
                                            resp_tx,
                                        ))
                                        .await
                                    {
                                        // todo event error,
                                        // we're gonna ignore it and continue by sending default motd.
                                    }

                                    if let Ok(res) = resp_rx.await {
                                        // this was a motd event,
                                        // lets send the pong!

                                        // get the motd from the server event otherwise use defaults.
                                        match res {
                                            ServerEventResponse::RefreshMotd(m) => motd = m,
                                            _ => {}
                                        };
                                    }

                                    // unconnected pong signature is different if MCPE is specified.
                                    let resp = UnconnectedPong {
                                        timestamp: raknet_start(),
                                        server_id,
                                        magic: Magic::new(),
                                        #[cfg(feature = "mcpe")]
                                        motd,
                                    };

                                    send_packet_to_socket(&socket, resp.into(), origin).await;
                                    continue;
                                }
                                OfflinePacket::OpenConnectRequest(pk) => {
                                    // todo make a constant for this
                                    if pk.protocol != 10_u8 {
                                        let resp = IncompatibleProtocolVersion {
                                            protocol: pk.protocol,
                                            magic: Magic::new(),
                                            server_id,
                                        };

                                        send_packet_to_socket(&socket, resp.into(), origin).await;
                                        continue;
                                    }

                                    let resp = OpenConnectReply {
                                        server_id,
                                        // todo allow encryption, find out if this is MCPE specific
                                        security: false,
                                        magic: Magic::new(),
                                        // todo make this configurable, this is sent to the client to change
                                        // it's mtu size, right now we're using what the client prefers.
                                        // however in some cases this may not be the preferred use case, for instance
                                        // on servers with larger worlds, you may want a larger mtu size
                                        mtu_size: pk.mtu_size,
                                    };
                                    send_packet_to_socket(&socket, resp.into(), origin).await;
                                }
                                OfflinePacket::SessionInfoRequest(pk) => {
                                    let mut sessions = connections.lock().await;
                                    let resp = SessionInfoReply {
                                        server_id,
                                        client_address: origin,
                                        magic: Magic::new(),
                                        mtu_size: pk.mtu_size,
                                        security: false,
                                    };

                                    // update the sessions mtuSize, this is referred to internally, we also will send this event to the client
                                    // event channel. However we are not expecting a response.
                                    drop(
                                        sessions.get_mut(&origin).unwrap().0.mtu_size = pk.mtu_size,
                                    );

                                    let (resp_tx, resp_rx) =
                                        oneshot::channel::<ServerEventResponse>();
                                    sessions[&origin]
                                        .2
                                        .send((ServerEvent::SetMtuSize(pk.mtu_size), resp_tx));

                                    send_packet_to_socket(&socket, resp.into(), origin);
                                }
                                _ => {
                                    // everything else should be sent to the socket
                                }
                            }
                        }
                        Payload::Online(pk) => {
                            // online packets need to be handled, but not yet :o
                        }
                    }
                } else {
                    // Not a valid raknet packet.
                    // Ignore the payload.
                }
            }
        });

        return Ok(());
    }
}

async fn send_packet_to_socket(socket: &Arc<UdpSocket>, packet: Packet, origin: SocketAddr) {
    if let Err(e) = socket
        .send_to(&mut packet.parse().unwrap()[..], origin)
        .await
    {
        // todo debug
    }
}

pub(crate) fn raknet_start() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
