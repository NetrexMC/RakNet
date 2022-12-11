/// Server events module. Handles things like updating the MOTD
/// for certain connections. This is a notifier channel.
pub mod event;

use std::collections::HashMap;
use std::{net::SocketAddr, sync::Arc};

use binary_utils::Streamable;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, oneshot};
use tokio::sync::Mutex;

use crate::conn::Conn;
use crate::error::server::ServerError;
use crate::error::{self, server};
use crate::protocol::Magic;
use crate::protocol::mcpe::motd::Motd;
use crate::protocol::packet::offline::{OfflinePacket, UnconnectedPong, IncompatibleProtocolVersion, OpenConnectReply};
use crate::protocol::packet::{Packet, Payload};

use self::event::ServerEvent;

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

    recv_comm: mpsc::Receiver<Conn>,
    send_comm: mpsc::Sender<Conn>,
    recv_evnt: mpsc::Receiver<(ServerEvent, oneshot::Sender<ServerEvent>)>,
    send_evnt: mpsc::Sender<(ServerEvent, oneshot::Sender<ServerEvent>)>,
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
        let (send_evnt, recv_evnt) = mpsc::channel::<(ServerEvent, oneshot::Sender<ServerEvent>)>(10);

        let listener = Self {
            sock: Some(Arc::new(sock)),
            id: server_id,
            motd,
            send_comm,
            recv_comm,
            send_evnt,
            recv_evnt,
            serving: false,
            // connections: Arc::new(Mutex::new(HashMap::new()))
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
        let ev_s = self.send_evnt.clone();

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
                    // if this is an offline packet, we can retrieve the buffer we should send.
                    match packet.payload {
                        Payload::Offline(pk) => {
                            // Offline packets are not buffered to the user.
                            // The reason for this is because we don't wish for the user to be able to disrupt
                            // raknet protocol, and handshaking.
                            match pk {
                                OfflinePacket::UnconnectedPing(_) => {
                                    let (resp_tx, resp_rx) = oneshot::channel::<ServerEvent>();
                                    let mut motd: Motd = motd_default.clone();

                                    ev_s.send((ServerEvent::RefreshMotdRequest(origin, motd.clone()), resp_tx)).await.ok().unwrap();

                                    if let Ok(res) = resp_rx.await {
                                        // this was a motd event,
                                        // lets send the pong!

                                        // get the motd from the server event otherwise use defaults.
                                        match res {
                                            ServerEvent::RefreshMotd(m) => motd = m,
                                            _ => {}
                                        };
                                    }

                                    // unconnected pong signature is different if MCPE is specified.
                                    let resp = UnconnectedPong {
                                        timestamp: raknet_start(),
                                        server_id,
                                        magic: Magic::new(),
                                        #[cfg(feature = "mcpe")]
                                        motd
                                    };

                                    send_packet_to_socket(&socket, resp.into(), origin).await;
                                    continue;
                                },
                                OfflinePacket::OpenConnectRequest(pk) => {
                                    // todo make a constant for this
                                    if pk.protocol != 10_u8 {
                                        let resp = IncompatibleProtocolVersion {
                                            protocol: pk.protocol,
                                            magic: Magic::new(),
                                            server_id
                                        };

                                        send_packet_to_socket(&socket, resp.into(), origin).await;
                                        continue;
                                    }

                                    let resp = OpenConnectReply {
                                        server_id,
                                        // todo allow encryption, find out if this is MCPE specific
                                        security: false,
                                        magic: Magic::new(),
                                        mtu_size: pk.mtu_size
                                    };
                                    send_packet_to_socket(&socket, resp.into(), origin).await;
                                },
                                _ => {
                                    // everything else shuold be sent to the socket
                                }
                            }
                        },
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
    if let Err(e) = socket.send_to(&mut packet.parse().unwrap()[..], origin).await {
        // todo debug
    }
}

pub(crate) fn raknet_start() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}