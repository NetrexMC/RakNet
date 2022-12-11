use std::collections::HashMap;
use std::{net::SocketAddr, sync::Arc};

use binary_utils::Streamable;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;

use crate::conn::Conn;
use crate::error::server::ServerError;
use crate::error::{self, server};
use crate::protocol::mcpe::motd::Motd;
use crate::protocol::packet::offline::OfflinePacket;
use crate::protocol::packet::Packet;

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

    recv_comm: Receiver<Conn>,
    send_comm: Sender<Conn>,
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

        let (send_comm, recv_comm) = channel::<Conn>(100);

        let listener = Self {
            sock: Some(Arc::new(sock)),
            id: server_id,
            motd,
            send_comm,
            recv_comm,
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

        tokio::spawn(async move {
            // We allocate here to prevent constant allocation of this array
            let mut buf: [u8; 2048] = [0; 2048];

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

                // Do a quick status check to see if it's an offline packet
                if let Ok(packet) = Packet::compose(&mut buf, &mut 0) {
                    // get the socket meta_data from
                } else {
                    // Not a valid raknet packet.
                    // Ignore the payload.
                }
            }
        });

        return Ok(());
    }
}
