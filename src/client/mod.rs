pub mod handshake;

use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, AtomicU64},
        Arc,
    },
    time::Duration,
};

#[cfg(feature = "async_std")]
use async_std::{
    channel::{bounded, Receiver, RecvError, Sender},
    net::UdpSocket,
    sync::{Mutex, RwLock},
    task::{self, sleep, JoinHandle},
};
use binary_utils::Streamable;
#[cfg(feature = "async_tokio")]
use tokio::{
    net::UdpSocket,
    sync::{
        mpsc::{channel as bounded, Receiver, Sender},
        Mutex, RwLock,
    },
    task::{self, JoinHandle},
    time::sleep,
};

use crate::{
    connection::{
        queue::{send, RecvQueue, SendQueue},
        state::ConnectionState,
    },
    error::client::ClientError,
    protocol::{
        ack::Ack,
        packet::{offline::OfflinePacket, online::ConnectedPing, Packet, Payload},
        reliability::Reliability,
    },
    rakrs_debug,
    server::{current_epoch, PossiblySocketAddr},
};

pub const DEFAULT_MTU: u16 = 1400;

use self::handshake::{ClientHandshake, HandshakeStatus};

/// This struct is used to connect to RakNet servers.
/// To start a connection, use `Client::connect()`.
pub struct Client {
    /// The connection state of the client.
    pub(crate) state: Arc<Mutex<ConnectionState>>,
    /// The send queue is used internally to send packets to the server.
    send_queue: Option<Arc<RwLock<SendQueue>>>,
    /// The receive queue is used internally to receive packets from the server.
    /// This is read from before sending
    recv_queue: Arc<Mutex<RecvQueue>>,
    /// The network recieve channel is used to receive raw packets from the server.
    network_recv: Option<Arc<Mutex<Receiver<Vec<u8>>>>>,
    /// The internal channel that is used to dispatch packets to a higher level.
    internal_recv: Receiver<Vec<u8>>,
    internal_send: Sender<Vec<u8>>,
    /// A list of tasks that are killed when the connection drops.
    tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// A notifier for when the client should kill threads.
    closed: Arc<AtomicBool>,
    /// A int for the last time a packet was received.
    recv_time: Arc<AtomicU64>,
    /// The maximum packet size that can be sent to the server.
    mtu: u16,
    /// The RakNet version of the client.
    version: u8,
    /// The internal client id of the client.
    id: u64,
}

impl Client {
    /// Creates a new client.
    /// > Note: This does not start a connection. You must use `Client::connect()` to start a connection.
    pub fn new(version: u8, mtu: u16) -> Self {
        let (internal_send, internal_recv) = bounded::<Vec<u8>>(10);
        Self {
            state: Arc::new(Mutex::new(ConnectionState::Offline)),
            send_queue: None,
            recv_queue: Arc::new(Mutex::new(RecvQueue::new())),
            network_recv: None,
            mtu,
            version,
            tasks: Arc::new(Mutex::new(Vec::new())),
            closed: Arc::new(AtomicBool::new(false)),
            recv_time: Arc::new(AtomicU64::new(0)),
            internal_recv,
            internal_send,
            id: rand::random::<u64>(),
        }
    }

    /// Connects to a RakNet server.
    /// > Note: This is an async function. You must await it.
    pub async fn connect<Addr: for<'a> Into<PossiblySocketAddr<'a>>>(
        &mut self,
        addr: Addr,
    ) -> Result<(), ClientError> {
        if self.state.lock().await.is_available() {
            return Err(ClientError::AlreadyOnline);
        }

        let addrr: PossiblySocketAddr = addr.into();
        let address: SocketAddr = match addrr.to_socket_addr() {
            Some(a) => a,
            None => {
                rakrs_debug!("Invalid address provided");
                return Err(ClientError::AddrBindErr);
            }
        };

        let sock = match UdpSocket::bind(address).await {
            Ok(s) => s,
            Err(e) => {
                rakrs_debug!("Failed to bind to address: {}", e);
                return Err(ClientError::Killed);
            }
        };

        let socket = Arc::new(sock);
        let send_queue = Arc::new(RwLock::new(SendQueue::new(
            self.mtu,
            12000,
            5,
            socket.clone(),
            address,
        )));

        self.send_queue = Some(send_queue.clone());
        let (net_send, net_recv) = bounded::<Vec<u8>>(10);

        self.network_recv = Some(Arc::new(Mutex::new(net_recv)));

        let closer = self.closed.clone();

        // before we even start the connection, we need to complete the handshake
        let handshake =
            ClientHandshake::new(socket.clone(), self.id as i64, self.version, self.mtu, 5).await;

        if handshake != HandshakeStatus::Completed {
            rakrs_debug!("Failed to complete handshake: {:?}", handshake);
            return Err(ClientError::Killed);
        }

        let socket_task = task::spawn(async move {
            let mut buf: [u8; 2048] = [0; 2048];

            loop {
                if closer.load(std::sync::atomic::Ordering::Relaxed) {
                    rakrs_debug!(true, "[CLIENT] Network recv task closed");
                    break;
                }

                let length: usize;

                let recv = socket.recv(&mut buf).await;
                match recv {
                    Ok(l) => length = l,
                    Err(e) => {
                        rakrs_debug!(true, "[CLIENT] Failed to receive packet: {}", e);
                        continue;
                    }
                }

                // no assertions because this is a client
                // this allows the user to customize their own packet handling
                if let Err(_) = net_send.send(buf[..length].to_vec()).await {
                    rakrs_debug!(true, "[CLIENT] Failed to send packet to network recv channel. Is the client closed?");
                }
            }
        });

        let recv_task = self.init_recv_task().await?;
        let tick_task = self.init_connect_tick(send_queue.clone()).await?;

        // Responsible for the raw socket
        self.push_task(socket_task).await;
        // Responsible for digesting messages from the network
        self.push_task(recv_task).await;
        // Responsible for sending packets to the server and keeping the connection alive
        self.push_task(tick_task).await;

        Ok(())
    }

    /// Updates the client state
    pub async fn update_state(&mut self, new_state: ConnectionState) {
        let mut state = self.state.lock().await;
        *state = new_state;
    }

    async fn push_task(&mut self, task: JoinHandle<()>) {
        self.tasks.lock().await.push(task);
    }

    async fn init_recv_task(&mut self) -> Result<JoinHandle<()>, ClientError> {
        let net_recv = match self.network_recv {
            Some(ref n) => n.clone(),
            None => {
                rakrs_debug!("[CLIENT] Network recv channel is not initialized");
                return Err(ClientError::Killed);
            }
        };

        let recv_queue = self.recv_queue.clone();
        let internal_sender = self.internal_send.clone();
        let closed = self.closed.clone();
        let state = self.state.clone();
        let version = self.version;

        let r = task::spawn(async move {
            loop {
                let pk_recv = net_recv.lock().await.recv().await;
                if closed.load(std::sync::atomic::Ordering::Relaxed) == true {
                    rakrs_debug!("[CLIENT] The internal network recieve channel has been killed.");
                    break;
                }

                if let Err(_) = pk_recv {
                    rakrs_debug!(true, "[CLIENT] Failed to recieve anything on netowrk channel, is there a sender?");
                    continue;
                }

                let buffer = pk_recv.unwrap();
            }
        });
        Ok(r)
    }

    /// This is an internal function that initializes the client connection.
    /// This is called by `Client::connect()`.
    async fn init_connect_tick(
        &mut self,
        send_queue: Arc<RwLock<SendQueue>>,
    ) -> Result<task::JoinHandle<()>, ClientError> {
        // verify that the client is offline
        if self.state.lock().await.is_available() {
            return Err(ClientError::AlreadyOnline);
        }

        let closer = self.closed.clone();
        let recv_queue = self.recv_queue.clone();
        let state = self.state.clone();
        let last_recv = self.recv_time.clone();
        let mut last_ping: u16 = 0;

        let t = task::spawn(async move {
            loop {
                sleep(Duration::from_millis(50)).await;

                if closer.load(std::sync::atomic::Ordering::Relaxed) {
                    rakrs_debug!(true, "[CLIENT] Connect tick task closed");
                    break;
                }

                let recv = last_recv.load(std::sync::atomic::Ordering::Relaxed);
                let mut state = state.lock().await;

                if *state == ConnectionState::Disconnected {
                    rakrs_debug!(
                        true,
                        "[CLIENT] Client is disconnected. Closing connect tick task"
                    );
                    closer.store(true, std::sync::atomic::Ordering::Relaxed);
                    break;
                }

                if *state == ConnectionState::Connecting {
                    rakrs_debug!(
                        true,
                        "[CLIENT] Client is not fully connected to the server yet."
                    );
                    continue;
                }

                if recv + 20000 <= current_epoch() {
                    *state = ConnectionState::Disconnected;
                    rakrs_debug!(true, "[CLIENT] Client timed out. Closing connection...");
                    closer.store(true, std::sync::atomic::Ordering::Relaxed);
                    break;
                }

                if recv + 15000 <= current_epoch() && state.is_reliable() {
                    *state = ConnectionState::TimingOut;
                    rakrs_debug!(true, "[CLIENT] Connection is timing out, sending a ping!",);
                }

                let mut send_q = send_queue.write().await;
                let mut recv_q = recv_queue.lock().await;

                if last_ping >= 3000 {
                    let ping = ConnectedPing {
                        time: current_epoch() as i64,
                    };
                    if let Ok(_) = send_q
                        .send_packet(ping.into(), Reliability::Reliable, true)
                        .await
                    {}
                    last_ping = 0;
                } else {
                    last_ping += 50;
                }

                send_q.update().await;

                // Flush the queue of acks and nacks, and respond to them
                let ack = Ack::from_records(recv_q.ack_flush(), false);
                if ack.records.len() > 0 {
                    if let Ok(p) = ack.parse() {
                        send_q.send_stream(&p).await;
                    }
                }

                // flush nacks from recv queue
                let nack = Ack::from_records(recv_q.nack_queue(), true);
                if nack.records.len() > 0 {
                    if let Ok(p) = nack.parse() {
                        send_q.send_stream(&p).await;
                    }
                }
            }
        });
        Ok(t)
    }
}
