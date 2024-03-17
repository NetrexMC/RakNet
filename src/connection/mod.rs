//! This module contains the logic to handle a connection or "peer" to the server.
//! This is used internally by the server, and is not intended to be used by the end user outside
//! of the server.
//!
//! This module contains the following:
//! - [`Connection`]: The connection struct, which is used to hold the connection state.
//! - [`ConnectionState`]: The connection state enum, which is used to hold the state of the connection.
//! - [`ConnectionMeta`]: The connection meta struct, which is used to hold the meta information of the connection.
//!
//! This module also contains the following submodules:
//! - [`controller`]: The controller submodule, which is used to handle relability of the connection.
//! - [`queue`]: The queue submodule, which is used to handle the connection queues.
//! - [`state`]: The state submodule, which is used to handle the connection state.
//!
//! # Example
//! This is a snippet of code you would use after you've accepted a connection from the server with
//! [`Listener::accept()`].
//!
//! ```ignore
//! use rakrs::connection::Connection;
//!
//! async fn handle(mut conn: Connection) {
//!     loop {
//!         // get a packet sent from the client
//!         if let Ok(pk) = conn.recv().await {
//!             println!("Got a connection packet {:?} ", pk);
//!         }
//!
//!         if !conn.state.lock().await.unwrap().is_available() {
//!             conn.disconnect("Client disconnected.", false);
//!             println!("Client disconnected!");
//!             break;
//!         }
//!     }
//! }
//! ```
//!
//! [`Listener::accept()`]: crate::server::Listener::accept
//! [`Connection`]: crate::connection::Connection
//! [`ConnectionState`]: crate::connection::state::ConnectionState
//! [`ConnectionMeta`]: crate::connection::ConnectionMeta
//! [`controller`]: crate::connection::controller
//! [`queue`]: crate::connection::queue
//! [`state`]: crate::connection::state
pub mod controller;
/// Necessary queues for the connection.
pub mod queue;
pub mod state;

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{atomic::AtomicU64, Arc},
    time::Duration,
};

use binary_util::interfaces::{Reader, Writer};

#[cfg(feature = "async_std")]
use async_std::{
    channel::{bounded, Receiver, RecvError, Sender},
    net::UdpSocket,
    sync::{Mutex, RwLock},
    task::{self, sleep, JoinHandle},
};
#[cfg(feature = "async_std")]
use futures::{select, FutureExt};
#[cfg(feature = "async_tokio")]
use tokio::{
    net::UdpSocket,
    select,
    sync::{
        mpsc::{channel as bounded, Receiver, Sender},
        Mutex, RwLock,
    },
    task::{self, JoinHandle},
    time::sleep,
};
#[cfg(feature = "async_tokio")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecvError {
    Closed,
    Timeout,
}
#[cfg(feature = "async_tokio")]
impl std::fmt::Display for RecvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecvError::Closed => write!(f, "RecvError: Channel is closed!"),
            RecvError::Timeout => write!(f, "RecvError: Timeout!"),
        }
    }
}

#[cfg(feature = "async_tokio")]
impl std::error::Error for RecvError {}

use crate::{
    notify::Notify,
    protocol::{
        ack::{Ack, Ackable, ACK, NACK},
        frame::FramePacket,
        packet::{
            offline::OfflinePacket,
            online::{ConnectedPing, ConnectedPong, ConnectionAccept, Disconnect, OnlinePacket},
        },
        reliability::Reliability,
    },
    rakrs_debug,
    server::current_epoch,
    util::to_address_token,
};

use self::{
    queue::{RecvQueue, SendQueue, SendQueueError},
    state::ConnectionState,
};
pub(crate) type ConnNetChan = Arc<Mutex<Receiver<Vec<u8>>>>;

#[derive(Debug, Clone, Copy)]
pub struct ConnMeta {
    /// This is important, and is stored within the server itself
    /// This value is 0 until the connection state is `Connecting`
    pub mtu_size: u16,
    /// The time this connection last sent any data. This will be used during server tick.
    pub recv_time: u64,
}

impl ConnMeta {
    pub fn new(mtu_size: u16) -> Self {
        Self {
            mtu_size,
            recv_time: current_epoch(),
        }
    }
}

/// The connection struct contains the logic for a connection to the server.
/// The following methods are the most important:
/// - [`Connection::recv()`]: This is used to recieve packets from the client.
/// - [`Connection::send()`]: This is used to send packets to the client.
/// - [`Connection::close()`]: This is used to disconnect the client.
///
/// <style>
/// .warning-2 {
///     background: rgba(255,240,76,0.34) !important;
///     padding: 0.75em;
///     border-left: 2px solid #fce811;
///     font-family: "Source Serif 4", NanumBarunGothic, serif;
///  }
///
/// .warning-2 code {
///     background: rgba(211,201,88,0.64) !important;
/// }
///
/// .notice-2 {
///     background: rgba(88, 211, 255, 0.34) !important;
///     padding: 0.75em;
///     border-left: 2px solid #4c96ff;
///     font-family: "Source Serif 4", NanumBarunGothic, serif;
/// }
///
/// .notice-2 code {
///     background: rgba(88, 211, 255, 0.64) !important;
/// }
/// </style>
///
/// <div class="warning-2">
///     <strong>Warning:</strong>
///     <p>
///         This struct does not provide an API for connecting to other peers, for that
///         you should use the
///         <a href="/rak-rs/latest/client/struct.Client.html">
///          Client
///         </a>
///         struct.
///     </p>
/// </div>
pub struct Connection {
    /// The address of the connection
    /// This is internally tokenized by rak-rs
    pub address: SocketAddr,
    pub state: Arc<Mutex<ConnectionState>>,
    /// The queue used to send packets back to the connection.
    send_queue: Arc<RwLock<SendQueue>>,
    /// The queue used to recieve packets, this is read from by the server.
    /// This is only used internally.
    recv_queue: Arc<Mutex<RecvQueue>>,
    /// The network channel, this is where the connection will be recieving it's packets.
    /// This is interfaced to provide the api for `Connection::recv()`
    internal_net_recv: ConnNetChan,
    /// A notifier for when the connection should close.
    /// This is used for absolute cleanup withtin the connection
    disconnect: Arc<Notify>,
    /// The event dispatcher for the connection.
    // evt_sender: Sender<(ServerEvent, oneshot::Sender<ServerEventResponse>)>,
    /// The event receiver for the connection.
    // evt_receiver: mpsc::Receiver<(ServerEvent, oneshot::Sender<ServerEventResponse>)>,
    /// The last time a packet was recieved. This is used to keep the connection from
    /// being in memory longer than it should be.
    recv_time: Arc<AtomicU64>,
    tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl Connection {
    /// Initializes a new Connection instance.
    pub async fn new(
        address: SocketAddr,
        socket: &Arc<UdpSocket>,
        net: Receiver<Vec<u8>>,
        notifier: Arc<Sender<SocketAddr>>,
        mtu: u16,
    ) -> Self {
        let (net_sender, net_receiver) = bounded::<Vec<u8>>(100);
        // let (evt_sender, evt_receiver) = mpsc::channel::<(ServerEvent, oneshot::Sender<ServerEventResponse>)>(10);
        let c = Self {
            address,
            send_queue: Arc::new(RwLock::new(SendQueue::new(
                mtu,
                12000,
                5,
                socket.clone(),
                address,
            ))),
            recv_queue: Arc::new(Mutex::new(RecvQueue::new())),
            internal_net_recv: Arc::new(Mutex::new(net_receiver)),
            // evt_sender,
            // evt_receiver,
            state: Arc::new(Mutex::new(ConnectionState::Unidentified)),
            // disconnect: Arc::new(Condvar::new()),
            disconnect: Arc::new(Notify::new()),
            recv_time: Arc::new(AtomicU64::new(current_epoch())),
            tasks: Arc::new(Mutex::new(Vec::new())),
        };

        let tk = c.tasks.clone();
        let mut tasks = tk.lock().await;
        tasks.push(c.init_tick(notifier));
        tasks.push(c.init_net_recv(net, net_sender));

        return c;
    }

    /// Initializes the client ticking process!
    pub(crate) fn init_tick(&self, notifier: Arc<Sender<SocketAddr>>) -> task::JoinHandle<()> {
        let address = self.address;
        let closer = self.disconnect.clone();
        let last_recv = self.recv_time.clone();
        let send_queue = self.send_queue.clone();
        let recv_queue = self.recv_queue.clone();
        let state = self.state.clone();
        let mut last_ping: u16 = 0;

        // initialize the event io
        // we initialize the ticking function here, it's purpose is to update the state of the current connection
        // while handling throttle
        return task::spawn(async move {
            loop {
                macro_rules! tick_body {
                    () => {
                        let recv = last_recv.load(std::sync::atomic::Ordering::Relaxed);
                        let mut cstate = state.lock().await;

                        if *cstate == ConnectionState::Disconnected {
                            rakrs_debug!(
                                true,
                                "[{}] Connection has been closed due to state!",
                                to_address_token(address)
                            );
                            // closer.notify_all();
                            closer.notify().await;
                            break;
                        }

                        if recv + 15 <= current_epoch() {
                            *cstate = ConnectionState::Disconnected;
                            rakrs_debug!(
                                true,
                                "[{}] Connection has been closed due to inactivity!",
                                to_address_token(address)
                            );
                            // closer.notify_all();
                            closer.notify().await;
                            break;
                        }

                        if recv + 10 <= current_epoch() && cstate.is_reliable() {
                            *cstate = ConnectionState::TimingOut;
                            rakrs_debug!(
                                true,
                                "[{}] Connection is timing out, sending a ping!",
                                to_address_token(address)
                            );
                        }

                        let mut sendq = send_queue.write().await;
                        let mut recv_q = recv_queue.lock().await;

                        if last_ping >= 3000 {
                            let ping = ConnectedPing {
                                time: current_epoch() as i64,
                            };
                            if let Ok(_) = sendq
                                .send_packet(ping.into(), Reliability::Reliable, true)
                                .await
                            {};
                            last_ping = 0;
                        } else {
                            last_ping += 50;
                        }

                        sendq.update().await;

                        // Flush the queue of acks and nacks, and respond to them
                        let ack = Ack::from_records(recv_q.ack_flush(), false);
                        if ack.records.len() > 0 {
                            if let Ok(p) = ack.write_to_bytes() {
                                sendq.send_stream(p.as_slice()).await;
                            }
                        }

                        // flush nacks from recv queue
                        let nack = Ack::from_records(recv_q.nack_queue(), true);
                        if nack.records.len() > 0 {
                            if let Ok(p) = nack.write_to_bytes() {
                                sendq.send_stream(p.as_slice()).await;
                            }
                        }
                    };
                }

                #[cfg(feature = "async_std")]
                select! {
                    _ = closer.wait().fuse() => {
                        rakrs_debug!(true, "[{}] [task: tick] Connection has been closed due to closer!", to_address_token(address));
                        break;
                    }
                    _ = sleep(Duration::from_millis(50)).fuse() => {
                       tick_body!();
                    }
                }

                #[cfg(feature = "async_tokio")]
                select! {
                    _ = closer.wait() => {
                        rakrs_debug!(true, "[{}] [task: tick] Connection has been closed due to closer!", to_address_token(address));
                        break;
                    }
                    _ = sleep(Duration::from_millis(50)) => {
                       tick_body!();
                    }
                }
            }

            #[cfg(feature = "async_std")]
            if let Ok(_) = notifier.send(address).await {
                rakrs_debug!(
                    true,
                    "[{}] [task: tick] Connection has been closed due to closer!",
                    to_address_token(address)
                );
            } else {
                rakrs_debug!(
                    true,
                    "[{}] [task: tick] Connection has been closed due to closer!",
                    to_address_token(address)
                );
            }

            #[cfg(feature = "async_tokio")]
            if let Ok(_) = notifier.send(address).await {
                rakrs_debug!(
                    true,
                    "[{}] [task: tick] Connection has been closed due to closer!",
                    to_address_token(address)
                );
            } else {
                rakrs_debug!(
                    true,
                    "[{}] [task: tick] Connection has been closed due to closer!",
                    to_address_token(address)
                );
            }
            rakrs_debug!(
                true,
                "[{}] Connection has been cleaned up!",
                to_address_token(address)
            );
        });
    }

    /// This function initializes the raw internal packet handling task!
    ///
    pub(crate) fn init_net_recv(
        &self,
        // THIS IS ONLY ACTIVATED ON STD
        #[cfg(feature = "async_std")] net: Receiver<Vec<u8>>,
        // ONLY ACTIVATED ON TOKIO
        #[cfg(feature = "async_tokio")] mut net: Receiver<Vec<u8>>,
        sender: Sender<Vec<u8>>,
    ) -> task::JoinHandle<()> {
        let recv_time = self.recv_time.clone();
        let recv_q = self.recv_queue.clone();
        let send_q = self.send_queue.clone();
        let disconnect = self.disconnect.clone();
        let state = self.state.clone();
        let address = self.address;

        return task::spawn(async move {
            loop {
                macro_rules! handle_payload {
                    ($payload: ident) => {
                        // We've recieved a payload!
                        recv_time.store(current_epoch(), std::sync::atomic::Ordering::Relaxed);
                        let mut cstate = state.lock().await;

                        if *cstate == ConnectionState::TimingOut {
                            rakrs_debug!(
                                "[{}] Connection is no longer timing out!",
                                to_address_token(address)
                            );
                            *cstate = ConnectionState::Connected;
                        }

                        drop(cstate);

                        let id = $payload[0];
                        match id {
                            // This is a frame packet.
                            // This packet will be handled by the recv_queue
                            0x80..=0x8d => {
                                if let Ok(pk) = FramePacket::read_from_slice(&$payload[..]) {
                                    let mut rq = recv_q.lock().await;

                                    if let Err(e) = rq.insert(pk) {
                                        rakrs_debug!(
                                            true,
                                            "[{}] Failed to insert frame packet! {:?}",
                                            to_address_token(address),
                                            e
                                        );
                                    };

                                    let buffers = rq.flush();

                                    for buffer in buffers {
                                        let res = Connection::process_packet(
                                            &buffer, &address, &sender, &send_q, &state,
                                        )
                                        .await;
                                        if let Ok(v) = res {
                                            if v == true {
                                                // DISCONNECT
                                                // disconnect.close();
                                                rakrs_debug!(true, "[{}] Connection::process_packet returned true!", to_address_token(address));
                                                disconnect.notify().await;
                                                break;
                                            }
                                        }
                                        if let Err(e) = res {
                                            rakrs_debug!(
                                                "[{}] Failed to process packet: {:?}!",
                                                to_address_token(address),
                                                e
                                            );
                                        };
                                    }

                                    drop(rq);
                                } else {
                                    rakrs_debug!(
                                        true,
                                        "[{}] Failed to parse frame packet!",
                                        to_address_token(address)
                                    );
                                }
                            }
                            NACK => {
                                // Validate this is a nack packet

                                if let Ok(nack) = Ack::read_from_slice(&$payload[..]) {
                                    // The client acknowledges it did not recieve these packets
                                    // We should resend them.
                                    let mut sq = send_q.write().await;
                                    let resend = sq.nack(nack);

                                    if resend.len() > 0 {
                                        for packet in resend {
                                            if let Ok(buffer) = packet.write_to_bytes() {
                                                if let Err(_) = sq.insert(buffer.as_slice(), Reliability::Unreliable, true, Some(0)).await {
                                                    rakrs_debug!(
                                                        true,
                                                        "[{}] Failed to insert packet into send queue!",
                                                        to_address_token(address)
                                                    );
                                                }
                                            } else {
                                                rakrs_debug!(
                                                    true,
                                                    "[{}] Failed to send packet to client (parsing failed)!",
                                                    to_address_token(address)
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            ACK => {
                                // first lets validate this is an ack packet
                                if let Ok(ack) = Ack::read_from_slice(&$payload[..]) {
                                    // The client acknowledges it recieved these packets
                                    // We should remove them from the queue.
                                    let mut sq = send_q.write().await;
                                    sq.ack(ack.clone());
                                    drop(sq);
                                    recv_q.lock().await.ack(ack);
                                }
                            }
                            _ => {
                                rakrs_debug!(
                                    "[{}] Unknown RakNet packet recieved (Or packet is sent out of scope).",
                                    to_address_token(address)
                                );
                            }
                        };
                    };
                }

                #[cfg(feature = "async_std")]
                select! {
                    _ = disconnect.wait().fuse() => {
                        rakrs_debug!(true, "[{}] [task: net_recv] Connection has been closed due to closer!", to_address_token(address));
                        break;
                    }
                    res = net.recv().fuse() => {
                        match res {
                            Ok(payload) => {
                                handle_payload!(payload);
                            }
                            _ => continue,
                        }
                    }
                };

                #[cfg(feature = "async_tokio")]
                select! {
                    _ = disconnect.wait() => {
                        rakrs_debug!(true, "[{}] [task: net_recv] Connection has been closed due to closer!", to_address_token(address));
                        break;
                    }
                    res = net.recv() => {
                        match res {
                            Some(payload) => {
                                handle_payload!(payload);
                            }
                            _ => continue,
                        }
                    }
                };
            }
        });
    }

    pub(crate) async fn process_packet(
        buffer: &[u8],
        address: &SocketAddr,
        sender: &Sender<Vec<u8>>,
        send_q: &Arc<RwLock<SendQueue>>,
        state: &Arc<Mutex<ConnectionState>>,
    ) -> Result<bool, ()> {
        if buffer.len() < 1 {
            rakrs_debug!("[{}] Got packet: {}", to_address_token(*address), buffer[0]);
        }
        if let Ok(online_packet) = OnlinePacket::read_from_slice(&buffer) {
            match online_packet {
                OnlinePacket::ConnectedPing(pk) => {
                    let response = ConnectedPong {
                        ping_time: pk.time,
                        pong_time: current_epoch() as i64,
                    };
                    let mut q = send_q.write().await;
                    if let Ok(_) = q
                        .send_packet(response.into(), Reliability::Reliable, true)
                        .await
                    {
                        return Ok(false);
                    } else {
                        rakrs_debug!(
                            true,
                            "[{}] Failed to send ConnectedPong packet!",
                            to_address_token(*address)
                        );
                        return Err(());
                    }
                }
                OnlinePacket::ConnectedPong(_pk) => {
                    // do nothing rn
                    // TODO: add ping calculation
                    return Ok(false);
                }
                OnlinePacket::ConnectionRequest(pk) => {
                    let internal_ids = vec![
                        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)), 19132),
                        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)), 19133),
                        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)), 19134),
                        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)), 19135),
                        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)), 19136),
                    ];
                    let response = ConnectionAccept {
                        system_index: 0,
                        client_address: *address,
                        internal_ids,
                        request_time: pk.time,
                        timestamp: current_epoch() as i64,
                    };
                    let mut q = send_q.write().await;
                    *state.lock().await = ConnectionState::Connecting;
                    if let Ok(_) = q
                        .send_packet(response.clone().into(), Reliability::Reliable, true)
                        .await
                    {
                        return Ok(false);
                    } else {
                        rakrs_debug!(
                            true,
                            "[{}] Failed to send ConnectionAccept packet!",
                            to_address_token(*address)
                        );
                        return Err(());
                    }
                }
                OnlinePacket::Disconnect(_) => {
                    // Disconnect the client immediately.
                    // connection.disconnect("Client disconnected.", false);
                    return Ok(true);
                }
                OnlinePacket::LostConnection(_) => {
                    // Disconnect the client immediately.
                    // connection.disconnect("Client disconnected.", false);
                    rakrs_debug!(
                        "[{}] Client has lost connection, disconnecting client!",
                        to_address_token(*address)
                    );
                    return Ok(true);
                }
                OnlinePacket::NewConnection(_) => {
                    // if we are already connected, disconnect the client.
                    if *state.lock().await == ConnectionState::Connected {
                        rakrs_debug!(
                            true,
                            "[{}] Client is already connected, disconnecting client!",
                            to_address_token(*address)
                        );
                        return Ok(true);
                    }

                    *state.lock().await = ConnectionState::Connected;
                    return Ok(false);
                }
                _ => {
                    rakrs_debug!(
                        true,
                        "[{}] Forwarding packet to socket!\n{:?}",
                        to_address_token(*address),
                        buffer
                    );
                    if let Err(_) = sender.send(buffer.to_vec()).await {
                        rakrs_debug!(
                            "[{}] Failed to to forward packet to recv channel...",
                            to_address_token(*address)
                        );
                        return Err(());
                    }
                    return Ok(false);
                }
            }
        } else if let Ok(_) = OfflinePacket::read_from_slice(&buffer) {
            *state.lock().await = ConnectionState::Disconnecting;
            rakrs_debug!(
                true,
                "[{}] Invalid protocol! Disconnecting client!",
                to_address_token(*address)
            );
            return Err(());
        }

        rakrs_debug!(
            true,
            "[{}] Either Game-packet or unknown packet, sending buffer to client...",
            to_address_token(*address)
        );
        if let Err(_) = sender.send(buffer.to_vec()).await {
            rakrs_debug!(
                "[{}] Failed to to forward packet to recv channel...",
                to_address_token(*address)
            );
            return Err(());
        }
        Ok(false)
    }

    /// This method is used to recieve packets from the client connection.
    /// Packets that are recieved here are packets sent by the peer expected
    /// to be handled by the server.
    ///
    /// This is where you would handle your packets from the client.
    ///
    /// # Example
    /// This is a snippet of code you would use after you've accepted a connection from the server with
    /// [`Listener::accept()`].
    ///
    /// ```ignore
    /// use rakrs::connection::Connection;
    ///
    /// async fn handle(mut conn: Connection) {
    ///     loop {
    ///         // get a packet sent from the client
    ///         if let Ok(pk) = conn.recv().await {
    ///             println!("Got a connection packet {:?} ", pk);
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn recv(&mut self) -> Result<Vec<u8>, RecvError> {
        #[allow(unused_mut)]
        let mut q = self.internal_net_recv.as_ref().lock().await;
        match q.recv().await {
            #[cfg(feature = "async_std")]
            Ok(packet) => Ok(packet),
            #[cfg(feature = "async_std")]
            Err(e) => Err(e),
            #[cfg(feature = "async_tokio")]
            Some(packet) => Ok(packet),
            #[cfg(feature = "async_tokio")]
            None => Err(RecvError::Closed),
        }
    }

    // /// Handle a RakNet Event. These are sent as they happen.
    // ///
    // /// EG:
    // /// ```ignore
    // /// let conn: Connection = Connection::new();
    // ///
    // /// while let Some((event, responder)) = conn.recv_ev {
    // ///     match event {
    // ///         ServerEvent::SetMtuSize(mtu) => {
    // ///             println!("client updated mtu!");
    // ///             responder.send(ServerEventResponse::Acknowledge);
    // ///         }
    // ///     }
    // /// }
    // /// ```
    // pub async fn recv_ev(
    //     &mut self,
    // ) -> Result<(ServerEvent, oneshot::Sender<ServerEventResponse>), ConnectionError> {
    //     match self.evt_receiver.recv().await {
    //         Some((server_event, event_responder)) => {
    //             return Ok((server_event, event_responder));
    //         }
    //         None => {
    //             if self.disconnect.is_closed() {
    //                 return Err(ConnectionError::Closed);
    //             }
    //             return Err(ConnectionError::EventDispatchError);
    //         }
    //     }
    // }

    pub async fn is_closed(&self) -> bool {
        !self.state.lock().await.is_available()
    }

    /// This method is used to send payloads to the connection. This method internally
    /// will encode your payload into a RakNet packet, and send it to the client.
    ///
    /// # Example
    /// This is a snippet of code you would use when you need to send a payload to the client.
    /// ```ignore
    /// use rakrs::connection::Connection;
    ///
    /// async fn send_payload(mut conn: Connection) {
    ///     conn.send(&[0x01, 0x02, 0x03], true).await.unwrap();
    /// }
    /// ```
    pub async fn send(&self, buffer: &[u8], immediate: bool) -> Result<(), SendQueueError> {
        let mut q = self.send_queue.write().await;
        rakrs_debug!("Send call, sending to write");
        if let Err(e) = q
            .insert(buffer, Reliability::ReliableOrd, immediate, Some(0))
            .await
        {
            return Err(e);
        }
        Ok(())
    }

    /// This method should be used when you are ready to disconnect the client.
    /// this method will attempt to send a disconnect packet to the client, and
    /// then close the connection.
    pub async fn close(&self) {
        rakrs_debug!(
            true,
            "[{}] Dropping connection!",
            to_address_token(self.address)
        );
        if let Err(_) = self
            .send(
                &OnlinePacket::Disconnect(Disconnect {})
                    .write_to_bytes()
                    .unwrap()
                    .as_slice(),
                true,
            )
            .await
        {
            rakrs_debug!(
                true,
                "[{}] Failed to send disconnect packet when closing!",
                to_address_token(self.address)
            );
        }
        let tasks = self.tasks.clone();

        for task in tasks.lock().await.drain(..) {
            #[cfg(feature = "async_std")]
            task.cancel().await;
            #[cfg(feature = "async_tokio")]
            task.abort();
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        futures_executor::block_on(async {
            if self.is_closed().await {
                return;
            }
            self.close().await;
        });
    }
}
