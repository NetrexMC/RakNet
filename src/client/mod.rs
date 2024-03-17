//! This module contains the client implementation of RakNet.
//! This module allows you to connect to a RakNet server, and
//! send and receive packets. This is the bare-bones implementation
//! for a RakNet client.
//!
//! # Getting Started
//! Connecting to a server is extremely easy with `rak-rs`, and can be done in a few lines of code.
//! In the following example we connect to `my_server.net:19132` and send a small packet, then wait
//! for a response, then close the connection when we're done.
//!
//! ```rust ignore
//! use rak_rs::client::{Client, DEFAULT_MTU};
//!
//! #[async_std::main]
//! async fn main() {
//!     let version: u8 = 10;
//!     let mut client = Client::new(version, DEFAULT_MTU);
//!
//!     if let Err(_) = client.connect("my_server.net:19132").await {
//!         println!("Failed to connect to server!");
//!         return;
//!     }
//!
//!     println!("Connected to server!");
//!
//!     client.send_ord(vec![254, 0, 1, 1], Some(1));
//!
//!     loop {
//!         let packet = client.recv().await.unwrap();
//!         println!("Received a packet! {:?}", packet);
//!         break;
//!     }
//!
//!     client.close().await;
//! }
//! ```
pub mod discovery;
pub mod handshake;
pub(crate) mod util;

use std::{
    net::SocketAddr,
    sync::{atomic::AtomicU64, Arc},
    time::Duration,
};

#[cfg(feature = "async_std")]
use async_std::{
    channel::{bounded, Receiver, RecvError, Sender},
    future::timeout,
    net::UdpSocket,
    sync::{Mutex, RwLock},
    task::{self, sleep, JoinHandle},
};

#[cfg(feature = "async_std")]
use futures::{select, FutureExt};

use binary_util::interfaces::{Reader, Writer};
use binary_util::io::ByteReader;

#[cfg(feature = "async_tokio")]
use tokio::{
    net::UdpSocket,
    select,
    sync::{
        mpsc::{channel as bounded, Receiver, Sender},
        Mutex, RwLock,
    },
    task::{self, JoinHandle},
    time::{sleep, timeout},
};

#[cfg(feature = "async_tokio")]
use crate::connection::RecvError;

use crate::{
    connection::{
        queue::{RecvQueue, SendQueue},
        state::ConnectionState,
    },
    error::client::ClientError,
    notify::Notify,
    protocol::{
        ack::{Ack, Ackable, ACK, NACK},
        frame::FramePacket,
        packet::{
            offline::{OfflinePacket, UnconnectedPing},
            online::{ConnectedPing, ConnectedPong, OnlinePacket},
            RakPacket,
        },
        reliability::Reliability,
        Magic,
    },
    rakrs_debug,
    server::{current_epoch, PossiblySocketAddr},
};

#[cfg(feature = "mcpe")]
use crate::protocol::mcpe::UnconnectedPong;
#[cfg(not(feature = "mcpe"))]
use crate::protocol::packet::offline::UnconnectedPong;

pub const DEFAULT_MTU: u16 = 1400;

use self::handshake::{ClientHandshake, HandshakeStatus};

/// This is the client implementation of RakNet.
/// This struct includes a few designated methods for sending and receiving packets.
/// - [`Client::send_ord()`] - Sends a packet with the [`Reliability::ReliableOrd`] reliability.
/// - [`Client::send_seq()`] - Sends a packet with the [`Reliability::ReliableSeq`] reliability.
/// - [`Client::send()`] - Sends a packet with a custom reliability.
///
/// # Ping Example
/// This is a simple example of how to use the client, this example will ping a server and print the latency.
/// ```rust ignore
/// use rak_rs::client::{Client, DEFAULT_MTU};
/// use std::net::UdpSocket;
/// use std::sync::Arc;
///
/// #[async_std::main]
/// async fn main() {
///     let mut socket = UdpSocket::bind("my_cool_server.net:19193").unwrap();
///     let socket_arc = Arc::new(socket);
///     if let Ok(pong) = Client::ping(socket).await {
///         println!("Latency: {}ms", pong.pong_time - pong.ping_time);
///     }
/// }
/// ```
///
/// # Implementation Example
/// In the following example we connect to `my_server.net:19132` and send a small packet, then wait
/// for a response, then close the connection when we're done.
///
/// ```rust ignore
/// use rak_rs::client::{Client, DEFAULT_MTU};
///
/// #[async_std::main]
/// async fn main() {
///     let version: u8 = 10;
///     let mut client = Client::new(version, DEFAULT_MTU);
///
///     if let Err(_) = client.connect("my_server.net:19132").await {
///         println!("Failed to connect to server!");
///         return;
///     }
///
///     println!("Connected to server!");
///
///     client.send_ord(vec![254, 0, 1, 1], Some(1));
///
///     loop {
///         let packet = client.recv().await.unwrap();
///         println!("Received a packet! {:?}", packet);
///         break;
///     }
///
///     client.close().await;
/// }
/// ```
///
/// [`Client::send_ord()`]: crate::client::Client::send_ord
/// [`Client::send_seq()`]: crate::client::Client::send_seq
/// [`Client::send()`]: crate::client::Client::send
pub struct Client {
    /// The connection state of the client.
    pub(crate) state: Arc<Mutex<ConnectionState>>,
    /// The send queue is used internally to send packets to the server.
    send_queue: Option<Arc<RwLock<SendQueue>>>,
    /// The receive queue is used internally to receive packets from the server.
    /// This is read from before sending
    recv_queue: Arc<Mutex<RecvQueue>>,
    /// The internal channel that is used to dispatch packets to a higher level.
    internal_recv: Receiver<Vec<u8>>,
    internal_send: Sender<Vec<u8>>,
    /// A list of tasks that are killed when the connection drops.
    tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// A notifier for when the client should kill threads.
    close_notifier: Arc<Notify>,
    /// A int for the last time a packet was received.
    recv_time: Arc<AtomicU64>,
    /// The time it takes to timeout a client.
    /// This is used to determine if a client is still connected.
    timeout: u64,
    /// This is the time it takes to timeout during handshaking.
    handshake_timeout: u16,
    /// The amount of handshake attempts to make before giving up.
    handshake_attempts: u8,
    /// The maximum packet size that can be sent to the server.
    mtu: u16,
    /// The RakNet version of the client.
    version: u8,
    /// The internal client id of the client.
    id: u64,
}

impl Client {
    /// Creates a new client.
    /// > Note: This does not start a connection. You must use [Client::connect()] to start a connection.
    ///
    /// # Example
    /// ```rust ignore
    /// use rak_rs::client::Client;
    ///
    /// let mut client = Client::new(10, 1400);
    /// ```
    ///
    /// [Client::connect()]: crate::client::Client::connect
    pub fn new(version: u8, mtu: u16) -> Self {
        let (internal_send, internal_recv) = bounded::<Vec<u8>>(10);
        Self {
            state: Arc::new(Mutex::new(ConnectionState::Offline)),
            send_queue: None,
            recv_queue: Arc::new(Mutex::new(RecvQueue::new())),
            mtu,
            version,
            tasks: Arc::new(Mutex::new(Vec::new())),
            close_notifier: Arc::new(Notify::new()),
            recv_time: Arc::new(AtomicU64::new(0)),
            internal_recv,
            internal_send,
            id: rand::random::<u64>(),
            timeout: 5000,
            handshake_timeout: 5000,
            handshake_attempts: 5,
        }
    }

    /// Changes the amount of time it takes to timeout a client. (in seconds)
    /// This is set to 20s by default.
    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = timeout;
        self
    }

    /// Changes the amount of time it takes to timeout during handshaking. (in seconds)
    /// This is set to 5s by default.
    pub fn with_handshake_timeout(mut self, timeout: u16) -> Self {
        self.handshake_timeout = timeout;
        self
    }

    /// Changes the amount of handshake attempts to make before giving up.
    /// This is set to 5 by default.
    pub fn with_handshake_attempts(mut self, attempts: u8) -> Self {
        self.handshake_attempts = attempts;
        self
    }

    /// This method should be used after [`Client::new()`] to start the connection.
    /// This method will start the connection, and will return a [`ClientError`] if the connection fails.
    ///
    /// # Example
    /// ```rust ignore
    /// use rak_rs::client::Client;
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     let mut client = Client::new(10, 1400);
    ///     if let Err(_) = client.connect("my_server.net:19132").await {
    ///         println!("Failed to connect to server!");
    ///         return;
    ///     }
    /// }
    /// ```
    ///
    /// [`Client::new()`]: crate::client::Client::new
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
                self.update_state(ConnectionState::Offline).await;
                return Err(ClientError::AddrBindErr);
            }
        };

        let sock = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(e) => {
                rakrs_debug!("Failed to bind to address: {}", e);
                self.update_state(ConnectionState::Offline).await;
                return Err(ClientError::Killed);
            }
        };

        rakrs_debug!(
            true,
            "[CLIENT] Attempting to connect to address: {}",
            address
        );

        let res = timeout(Duration::from_secs(5), sock.connect(address)).await;

        if res.is_err() {
            rakrs_debug!("[CLIENT] Failed to connect to address");
            self.close_notifier.notify().await;
            self.update_state(ConnectionState::Offline).await;
            return Err(ClientError::Killed);
        }

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

        let closer = self.close_notifier.clone();

        Self::internal_ping(socket.clone()).await?;

        self.update_state(ConnectionState::Unidentified).await;
        rakrs_debug!(true, "[CLIENT] Starting connection handshake");
        // before we even start the connection, we need to complete the handshake
        let handshake = ClientHandshake::new(
            socket.clone(),
            self.id as i64,
            self.version,
            self.mtu,
            self.handshake_attempts,
            self.handshake_timeout,
        )
        .await;

        if handshake != HandshakeStatus::Completed {
            rakrs_debug!("Failed to complete handshake: {:?}", handshake);
            self.update_state(ConnectionState::Offline).await;
            return Err(ClientError::HandshakeError(handshake));
        }
        self.update_state(ConnectionState::Identified).await;

        rakrs_debug!(true, "[CLIENT] Handshake completed!");

        let socket_task = task::spawn(async move {
            let mut buf: [u8; 2048] = [0; 2048];
            let notifier = closer.clone();

            loop {
                let length: usize;

                #[cfg(feature = "async_std")]
                select! {
                    killed = notifier.wait().fuse() => {
                        if killed {
                            rakrs_debug!(true, "[CLIENT] Socket task closed");
                            break;
                        }
                    }

                    recv = socket.recv(&mut buf).fuse() => {
                        match recv {
                            Ok(l) => length = l,
                            Err(e) => {
                                rakrs_debug!(true, "[CLIENT] Failed to receive packet: {}", e);
                                continue;
                            }
                        }
                        // no assertions because this is a client
                        // this allows the user to customize their own packet handling
                        // todo: the logic in the recv_task may be better here, as this is latent
                        if let Err(_) = net_send.send(buf[..length].to_vec()).await {
                            rakrs_debug!(true, "[CLIENT] Failed to send packet to network recv channel. Is the client closed?");
                        }
                    }
                };

                #[cfg(feature = "async_tokio")]
                select! {
                    killed = notifier.wait() => {
                        if killed {
                            rakrs_debug!(true, "[CLIENT] Socket task closed");
                            break;
                        }
                    }

                    recv = socket.recv(&mut buf) => {
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
                };
            }
        });

        let recv_task = self.init_recv_task(net_recv);
        let tisk_task = self.init_connect_tick(send_queue.clone());

        if let Err(e) = recv_task {
            rakrs_debug!(true, "[CLIENT] Failed to start recv task: {:?}", e);
            return Err(ClientError::Killed);
        }

        if let Err(e) = tisk_task {
            rakrs_debug!(true, "[CLIENT] Failed to start connect tick task: {:?}", e);
            return Err(ClientError::Killed);
        }

        let recv_task = recv_task.unwrap();
        let tisk_task = tisk_task.unwrap();

        if *self.state.lock().await != ConnectionState::Identified {
            return Err(ClientError::AlreadyOnline);
        }

        self.update_state(ConnectionState::Connected).await;

        let mut tasks = self.tasks.lock().await;

        // Responsible for the raw socket
        tasks.push(socket_task);
        // Responsible for digesting messages from the network
        tasks.push(recv_task);
        // Responsible for sending packets to the server and keeping the connection alive
        tasks.push(tisk_task);

        rakrs_debug!("[CLIENT] Client is now connected!");
        Ok(())
    }

    /// Updates the client state
    pub async fn update_state(&self, new_state: ConnectionState) {
        let mut state = self.state.lock().await;
        *state = new_state;
    }

    /// Todo: send disconnect packet.
    pub async fn close(&self) {
        self.update_state(ConnectionState::Disconnecting).await;
        let notifier = self.close_notifier.clone();
        notifier.notify().await;
        let mut tasks = self.tasks.lock().await;
        for task in tasks.drain(..) {
            #[cfg(feature = "async_std")]
            task.cancel().await;
            #[cfg(feature = "async_tokio")]
            task.abort();
        }
    }

    pub async fn send_ord(&self, buffer: &[u8], channel: u8) -> Result<(), ClientError> {
        if self.state.lock().await.is_available() {
            let mut send_q = self.send_queue.as_ref().unwrap().write().await;
            if let Err(send) = send_q
                .insert(buffer, Reliability::ReliableOrd, false, Some(channel))
                .await
            {
                rakrs_debug!(true, "[CLIENT] Failed to insert packet into send queue!");
                return Err(ClientError::SendQueueError(send));
            }
            Ok(())
        } else {
            rakrs_debug!(
                true,
                "[CLIENT] Client is not connected! State: {:?}",
                self.state.lock().await
            );
            Err(ClientError::NotListening)
        }
    }

    pub async fn send_seq(&self, buffer: &[u8], channel: u8) -> Result<(), ClientError> {
        if self.state.lock().await.is_available() {
            let mut send_q = self.send_queue.as_ref().unwrap().write().await;
            if let Err(send) = send_q
                .insert(buffer, Reliability::ReliableSeq, false, Some(channel))
                .await
            {
                rakrs_debug!(true, "[CLIENT] Failed to insert packet into send queue!");
                return Err(ClientError::SendQueueError(send));
            }
            Ok(())
        } else {
            Err(ClientError::Unavailable)
        }
    }

    pub async fn send(
        &self,
        buffer: &[u8],
        reliability: Reliability,
        channel: u8,
    ) -> Result<(), ClientError> {
        if self.state.lock().await.is_available() {
            let mut send_q = self.send_queue.as_ref().unwrap().write().await;
            if let Err(send) = send_q
                .insert(buffer, reliability, false, Some(channel))
                .await
            {
                rakrs_debug!(true, "[CLIENT] Failed to insert packet into send queue!");
                return Err(ClientError::SendQueueError(send));
            }
            Ok(())
        } else {
            Err(ClientError::Unavailable)
        }
    }

    /// ONLY USE IF YOU KNOW WHAT YOU ARE DOING
    /// THIS WILL BREAK CONGESTION CONTROL
    pub async fn send_raw(&self, buffer: &[u8]) -> Result<(), ClientError> {
        if self.state.lock().await.is_available() {
            let mut send_q = self.send_queue.as_ref().unwrap().write().await;
            if let Err(send) = send_q
                .insert(buffer, Reliability::Reliable, true, None)
                .await
            {
                rakrs_debug!(true, "[CLIENT] Failed to insert packet into send queue!");
                return Err(ClientError::SendQueueError(send));
            }
            Ok(())
        } else {
            Err(ClientError::Unavailable)
        }
    }

    /// Sends a packet immediately, bypassing the send queue.
    /// This is useful for things like player login and responses, however is discouraged
    /// for general use, due to congestion control.
    ///
    /// # Example
    /// ```rust ignore
    /// use rak_rs::client::Client;
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///    let mut client = Client::new(10, 1400);
    ///    if let Err(e) = client.connect("my_server.net:19132").await {
    ///        println!("Failed to connect! {}", e);
    ///        return;
    ///    }
    ///    // Sent immediately.
    ///    client.send_immediate(&[0, 1, 2, 3], Reliability::Reliable, 0).await.unwrap();
    /// }
    /// ```
    pub async fn send_immediate(
        &self,
        buffer: &[u8],
        reliability: Reliability,
        channel: u8,
    ) -> Result<(), ClientError> {
        if self.state.lock().await.is_available() {
            let mut send_q = self.send_queue.as_ref().unwrap().write().await;
            if let Err(send) = send_q
                .insert(buffer, reliability, true, Some(channel))
                .await
            {
                rakrs_debug!(true, "[CLIENT] Failed to insert packet into send queue!");
                return Err(ClientError::SendQueueError(send));
            }
            Ok(())
        } else {
            Err(ClientError::Unavailable)
        }
    }

    /// Attempts to flush the acknowledgement queue, this can be useful if you notice that
    /// the client stops responding to packets, or you are handling large chunks of data
    /// that is time sensitive.
    pub async fn flush_ack(&self) {
        let mut send_q = self.send_queue.as_ref().unwrap().write().await;
        let mut recv_q = self.recv_queue.lock().await;
        // Flush the queue of acks and nacks, and respond to them
        let ack = Ack::from_records(recv_q.ack_flush(), false);
        if ack.records.len() > 0 {
            if let Ok(p) = ack.write_to_bytes() {
                send_q.send_stream(p.as_slice()).await;
            }
        }

        // flush nacks from recv queue
        let nack = Ack::from_records(recv_q.nack_queue(), true);
        if nack.records.len() > 0 {
            if let Ok(p) = nack.write_to_bytes() {
                send_q.send_stream(p.as_slice()).await;
            }
        }
    }

    #[cfg(feature = "async_std")]
    pub async fn recv(&self) -> Result<Vec<u8>, RecvError> {
        select! {
            packet = self.internal_recv.recv().fuse() => {
                packet
            }
            _ = self.close_notifier.wait().fuse() => {
                Err(RecvError)
            }
        }
    }

    #[cfg(feature = "async_tokio")]
    pub async fn recv(&mut self) -> Result<Vec<u8>, RecvError> {
        tokio::select! {
            packet = self.internal_recv.recv() => {
                match packet {
                    Some(packet) => Ok(packet),
                    None => Err(RecvError::Closed),
                }
            },
            _ = self.close_notifier.wait() => {
                Err(RecvError::Closed)
            }
        }
    }

    /// Pings a server and returns the latency via [`UnconnectedPong`].
    ///
    /// [`UnconnectedPong`]: crate::protocol::packet::offline::UnconnectedPong
    ///
    /// # Example
    /// ```rust ignore
    /// use rak_rs::client::Client;
    /// use std::sync::Arc;
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     if let Ok(pong) = Client::ping("zeqa.net:19132").await {
    ///         println!("Latency: {}ms", pong.pong_time - pong.ping_time);
    ///     }
    /// }
    /// ```
    pub async fn ping<I: for<'a> Into<PossiblySocketAddr<'a>>>(
        address: I,
    ) -> Result<UnconnectedPong, ClientError> {
        let a: PossiblySocketAddr = address.into();
        let address_r: Option<SocketAddr> = a.to_socket_addr();

        if address_r.is_none() {
            return Err(ClientError::AddrBindErr);
        }

        let socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(_) => return Err(ClientError::AddrBindErr),
        };

        if let Err(e) = socket.connect(address_r.unwrap()).await {
            rakrs_debug!(
                true,
                "[CLIENT] Failed to connect to ({}): {}",
                address_r.unwrap(),
                e
            );
            return Err(ClientError::Reset);
        }

        let socket_arc = Arc::new(socket);

        return Self::internal_ping(socket_arc).await;
    }

    /// Pings a server and returns the latency via [`UnconnectedPong`].
    ///
    /// [`UnconnectedPong`]: crate::protocol::packet::offline::UnconnectedPong
    ///
    /// # Example
    /// ```rust ignore
    /// use rak_rs::client::Client;
    /// use std::sync::Arc;
    /// use async_std::net::UdpSocket;
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     let mut socket = UdpSocket::bind("my_cool_server.net:19193").unwrap();
    ///     let socket_arc = Arc::new(socket);
    ///     if let Ok(pong) = Client::ping(socket).await {
    ///         println!("Latency: {}ms", pong.pong_time - pong.ping_time);
    ///     }
    /// }
    /// ```
    pub(crate) async fn internal_ping(
        socket: Arc<UdpSocket>,
    ) -> Result<UnconnectedPong, ClientError> {
        let mut buf: [u8; 2048] = [0; 2048];
        let unconnected_ping = UnconnectedPing {
            timestamp: current_epoch(),
            magic: Magic::new(),
            client_id: rand::random::<i64>(),
        };

        if let Err(e) = socket
            .send(
                RakPacket::from(unconnected_ping.clone())
                    .write_to_bytes()
                    .unwrap()
                    .as_slice(),
            )
            .await
        {
            rakrs_debug!(true, "[CLIENT::PING]Failed to send ping packet!");
            rakrs_debug!(true, "[CLIENT::PING] Failed to send ping packet: {}", e);
            return Err(ClientError::ServerOffline);
        }

        loop {
            rakrs_debug!(true, "[CLIENT::PING] Waiting for pong packet...");
            if let Ok(recvd) = timeout(Duration::from_millis(10000), socket.recv(&mut buf)).await {
                match recvd {
                    Ok(l) => {
                        let mut reader = ByteReader::from(&buf[..l]);
                        let packet = match RakPacket::read(&mut reader) {
                            Ok(p) => p,
                            Err(_) => {
                                continue;
                            }
                        };

                        match packet {
                            RakPacket::Offline(offline) => match offline {
                                OfflinePacket::UnconnectedPong(pong) => {
                                    rakrs_debug!(true, "[CLIENT::PING] Received pong packet!");
                                    return Ok(pong);
                                }
                                _ => {
                                    rakrs_debug!(
                                        true,
                                        "[CLIENT::PING] Received unknown packet: {:?}",
                                        offline
                                    );
                                }
                            },
                            _ => {
                                rakrs_debug!(
                                    true,
                                    "[CLIENT::PING] Received unknown packet: {:?}",
                                    packet
                                );
                            }
                        }
                    }
                    Err(_) => {
                        rakrs_debug!(true, "[CLIENT::PING] Failed to recieve anything on netowrk channel, is there a sender?");
                        continue;
                    }
                }
            } else {
                rakrs_debug!(true, "[CLIENT::PING] Ping Failed, server did not respond!");
                return Err(ClientError::ServerOffline);
            }
        }
    }

    /// This should only be used when you need to verify that the client is still connected.
    /// Do not use this for anything else.
    pub async fn is_connected(&self) -> bool {
        self.state.lock().await.is_connected()
    }

    /// Returns the current state of the client.
    /// Not to be confused with [`Client::is_connected()`].
    ///
    /// [`Client::is_connected()`]: crate::client::Client::is_connected
    pub async fn get_state(&self) -> ConnectionState {
        *self.state.lock().await
    }

    /// Internal, this is the task that is responsible for receiving packets and dispatching
    /// them to the user.
    ///
    /// This is responsible for the user api on [`Client::recv()`].
    ///
    /// [`Client::recv()`]: crate::client::Client::recv
    fn init_recv_task(
        &self,
        #[allow(unused_mut)] mut net_recv: Receiver<Vec<u8>>,
    ) -> Result<JoinHandle<()>, ClientError> {
        let send_queue = match self.send_queue {
            Some(ref s) => s.clone(),
            None => {
                rakrs_debug!("[CLIENT] (recv_task) Send queue is not initialized");
                return Err(ClientError::Killed);
            }
        };

        let recv_queue = self.recv_queue.clone();
        let internal_sender = self.internal_send.clone();
        let closed = self.close_notifier.clone();
        let state = self.state.clone();
        let recv_time = self.recv_time.clone();

        return Ok(task::spawn(async move {
            'task_loop: loop {
                // #[cfg(feature = "async_tokio")]
                let closed_dispatch = closed.clone();

                macro_rules! recv_body {
                    ($pk_recv: expr) => {
                        #[cfg(feature = "async_std")]
                        if let Err(_) = $pk_recv {
                            rakrs_debug!(true, "[CLIENT] (recv_task) Failed to receive anything on network channel, is there a sender?");
                            continue;
                        }

                        #[cfg(feature = "async_tokio")]
                        if let None = $pk_recv {
                            rakrs_debug!(true, "[CLIENT] (recv_task) Failed to receive anything on network channel, is there a sender?");
                            continue;
                        }

                        recv_time.store(current_epoch(), std::sync::atomic::Ordering::Relaxed);

                        let mut client_state = state.lock().await;

                        if *client_state == ConnectionState::TimingOut {
                            rakrs_debug!(true, "[CLIENT] (recv_task) Client is no longer timing out!");
                            *client_state = ConnectionState::Connected;
                        }

                        if *client_state == ConnectionState::Disconnecting {
                            rakrs_debug!(true, "[CLIENT] (recv_task) Client is disconnecting!");
                            break;
                        }

                        // drop here so the lock isn't held for too long
                        drop(client_state);

                        let mut buffer = ByteReader::from($pk_recv.unwrap());

                        match buffer.as_slice()[0] {
                            0x80..=0x8d => {
                                if let Ok(frame_packet) = FramePacket::read(&mut buffer) {
                                    let mut recv_q = recv_queue.lock().await;
                                    if let Err(_) = recv_q.insert(frame_packet) {
                                        rakrs_debug!(
                                            true,
                                            "[CLIENT] Failed to push frame packet into send queue."
                                        );
                                    }

                                    let buffers = recv_q.flush();

                                    'buf_loop: for pk_buf_raw in buffers {
                                        let mut pk_buf = ByteReader::from(&pk_buf_raw[..]);
                                        if let Ok(rak_packet) = RakPacket::read(&mut pk_buf) {
                                            match rak_packet {
                                                RakPacket::Online(pk) => {
                                                    match pk {
                                                        OnlinePacket::ConnectedPing(pk) => {
                                                            let response = ConnectedPong {
                                                                ping_time: pk.time,
                                                                pong_time: current_epoch() as i64,
                                                            };
                                                            let mut q = send_queue.write().await;
                                                            if let Err(_) = q
                                                                .send_packet(
                                                                    response.into(),
                                                                    Reliability::Unreliable,
                                                                    true,
                                                                )
                                                                .await
                                                            {
                                                                rakrs_debug!(
                                                                    true,
                                                                    "[CLIENT] Failed to send pong packet!"
                                                                );
                                                            }
                                                            continue 'buf_loop;
                                                        }
                                                        OnlinePacket::ConnectedPong(_) => {
                                                            // todo: add ping time to client
                                                            rakrs_debug!(
                                                                true,
                                                                "[CLIENT] Received pong packet!"
                                                            );
                                                        }
                                                        OnlinePacket::Disconnect(_) => {
                                                            rakrs_debug!(
                                                                true,
                                                                "[CLIENT] Received disconnect packet!"
                                                            );
                                                            break 'task_loop;
                                                        }
                                                        _ => {
                                                            rakrs_debug!(
                                                                true,
                                                                "[CLIENT] Processing fault packet... {:#?}",
                                                                pk
                                                            );

                                                            if let Err(_) = internal_sender.send(pk_buf_raw).await {
                                                                rakrs_debug!(true, "[CLIENT] Failed to send packet to internal recv channel. Is the client closed?");
                                                            }
                                                        }
                                                    }
                                                },
                                                RakPacket::Offline(_) => {
                                                    rakrs_debug!("[CLIENT] Received offline packet after handshake! In future versions this will kill the client.");
                                                }
                                            }
                                        } else {
                                            // we send this packet
                                            if let Err(_) = internal_sender.send(pk_buf_raw).await {
                                                rakrs_debug!(true, "[CLIENT] Failed to send packet to internal recv channel. Is the client closed?");
                                            }
                                        }
                                    }
                                }
                            }
                            NACK => {
                                if let Ok(nack) = Ack::read(&mut buffer) {
                                    let mut send_q = send_queue.write().await;
                                    let to_resend = send_q.nack(nack);

                                    if to_resend.len() > 0 {
                                        for ack_packet in to_resend {
                                            if let Ok(buffer) = ack_packet.write_to_bytes() {
                                                if let Err(_) = send_q
                                                    .insert(buffer.as_slice(), Reliability::Unreliable, true, Some(0))
                                                    .await
                                                {
                                                    rakrs_debug!(
                                                        true,
                                                        "[CLIENT] Failed to insert ack packet into send queue!"
                                                    );
                                                }
                                            } else {
                                                rakrs_debug!(
                                                    true,
                                                    "[CLIENT] Failed to send packet to client (parsing failed)!",
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            ACK => {
                                if let Ok(ack) = Ack::read(&mut buffer) {
                                    let mut send_q = send_queue.write().await;
                                    send_q.ack(ack.clone());

                                    drop(send_q);

                                    recv_queue.lock().await.ack(ack);
                                }
                            }
                            _ => {
                                // we don't know what this is, so we're going to send it to the user, maybe
                                // this is a custom packet
                                if let Err(_) = internal_sender.send(buffer.as_slice().to_vec()).await {
                                    rakrs_debug!(true, "[CLIENT] Failed to send packet to internal recv channel. Is the client closed?");
                                }
                            }
                        }
                    };
                }

                #[cfg(feature = "async_std")]
                select! {
                    killed = closed_dispatch.wait().fuse() => {
                        if killed {
                            rakrs_debug!(true, "[CLIENT] Recv task closed");
                            break 'task_loop;
                        }
                    }
                    pk_recv = net_recv.recv().fuse() => {
                        recv_body!(pk_recv);
                    }
                }

                #[cfg(feature = "async_tokio")]
                select! {
                    killed = closed_dispatch.wait() => {
                        if killed {
                            rakrs_debug!(true, "[CLIENT] Recv task closed");
                            break 'task_loop;
                        }
                    }
                    pk_recv = net_recv.recv() => {
                        recv_body!(pk_recv);
                    }
                }
            }
        }));
    }

    /// This is an internal function that initializes the client connection.
    /// This is called by [`Client::connect()`].
    ///
    /// [`Client::connect()`]: crate::client::Client::connect
    fn init_connect_tick(
        &self,
        send_queue: Arc<RwLock<SendQueue>>,
    ) -> Result<task::JoinHandle<()>, ClientError> {
        // verify that the client is offline
        let closer = self.close_notifier.clone();
        let recv_queue = self.recv_queue.clone();
        let state = self.state.clone();
        let last_recv = self.recv_time.clone();
        let timeout_time = self.timeout;
        let mut last_ping: u16 = 0;

        return Ok(task::spawn(async move {
            loop {
                macro_rules! tick_body {
                    () => {
                        let recv = last_recv.load(std::sync::atomic::Ordering::Relaxed);
                        let mut state = state.lock().await;

                        if *state == ConnectionState::Disconnected {
                            rakrs_debug!(
                                true,
                                "[CLIENT] Client is disconnected. Closing connect tick task"
                            );
                            closer.notify().await;
                            break;
                        }

                        if *state == ConnectionState::Connecting {
                            rakrs_debug!(
                                true,
                                "[CLIENT] Client is not fully connected to the server yet."
                            );
                            continue;
                        }

                        if ((recv + timeout_time) <= current_epoch()) && recv != 0 {
                            *state = ConnectionState::Disconnected;
                            rakrs_debug!(true, "[CLIENT] Client timed out. Closing connection... last={}, current={}", recv, current_epoch());
                            closer.notify().await;
                            break;
                        }

                        let mut send_q = send_queue.write().await;
                        let mut recv_q = recv_queue.lock().await;

                        if (recv + (timeout_time / 2)) <= current_epoch() && state.is_reliable() {
                            *state = ConnectionState::TimingOut;
                            rakrs_debug!(
                                true,
                                "[CLIENT] Connection is timing out, sending a ping!",
                            );
                            let ping = ConnectedPing {
                                time: current_epoch() as i64,
                            };
                            if let Ok(_) = send_q
                                .send_packet(ping.into(), Reliability::Reliable, true)
                                .await
                            {}
                        }

                        if last_ping >= 500 {
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
                            if let Ok(p) = ack.write_to_bytes() {
                                send_q.send_stream(p.as_slice()).await;
                            }
                        }

                        // flush nacks from recv queue
                        let nack = Ack::from_records(recv_q.nack_queue(), true);
                        if nack.records.len() > 0 {
                            if let Ok(p) = nack.write_to_bytes() {
                                send_q.send_stream(p.as_slice()).await;
                            }
                        }
                    };
                }

                #[cfg(feature = "async_std")]
                select! {
                    _ = sleep(Duration::from_millis(50)).fuse() => {
                        tick_body!();
                    },
                    killed = closer.wait().fuse() => {
                        if killed {
                            rakrs_debug!(true, "[CLIENT] Connect tick task closed");
                            break;
                        }
                    }
                }

                #[cfg(feature = "async_tokio")]
                select! {
                    _ = sleep(Duration::from_millis(50)) => {
                        tick_body!();
                    },
                    killed = closer.wait() => {
                        if killed {
                            rakrs_debug!(true, "[CLIENT] Connect tick task closed");
                            break;
                        }
                    }
                }
            }
        }));
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // todo: There is DEFINITELY a better way to do this...
        futures_executor::block_on(async move { self.close_notifier.notify().await });
    }
}
