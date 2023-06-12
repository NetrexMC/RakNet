pub mod handshake;

use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, AtomicU64},
        Arc,
    },
    time::Duration,
};

#[cfg(feature = "async_tokio")]
use crate::connection::RecvError;
#[cfg(feature = "async_std")]
use async_std::future::timeout;
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
    time::{sleep, timeout},
};

use crate::{
    connection::{
        queue::{RecvQueue, SendQueue},
        state::ConnectionState,
    },
    error::client::ClientError,
    protocol::{
        ack::{Ack, Ackable, ACK, NACK},
        frame::FramePacket,
        packet::{
            offline::{OfflinePacket, UnconnectedPing},
            online::{ConnectedPing, ConnectedPong, OnlinePacket},
            Packet,
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

        let sock = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(e) => {
                rakrs_debug!("Failed to bind to address: {}", e);
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
            self.closed
                .store(false, std::sync::atomic::Ordering::Relaxed);
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

        self.network_recv = Some(Arc::new(Mutex::new(net_recv)));

        let closer = self.closed.clone();

        Self::ping(socket.clone()).await?;

        rakrs_debug!(true, "[CLIENT] Starting connection handshake");
        // before we even start the connection, we need to complete the handshake
        let handshake =
            ClientHandshake::new(socket.clone(), self.id as i64, self.version, self.mtu, 5).await;

        if handshake != HandshakeStatus::Completed {
            rakrs_debug!("Failed to complete handshake: {:?}", handshake);
            return Err(ClientError::Killed);
        }

        rakrs_debug!(true, "[CLIENT] Handshake completed!");

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
                // todo: the logic in the recv_task may be better here, as this is latent
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

    /// Todo: send disconnect packet.
    pub async fn close(&mut self) {
        self.update_state(ConnectionState::Disconnecting).await;
        self.closed
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let mut tasks = self.tasks.lock().await;
        for task in tasks.drain(..) {
            #[cfg(feature = "async_std")]
            task.cancel().await;
            #[cfg(feature = "async_tokio")]
            task.abort();
        }
    }

    pub async fn send_ord(&mut self, buffer: &[u8], channel: u8) -> Result<(), ClientError> {
        if self.state.lock().await.is_available() {
            let mut send_q = self.send_queue.as_ref().unwrap().write().await;
            if let Err(send) = send_q
                .insert(
                    buffer.to_vec(),
                    Reliability::ReliableOrd,
                    false,
                    Some(channel),
                )
                .await
            {
                rakrs_debug!(true, "[CLIENT] Failed to insert packet into send queue!");
                return Err(ClientError::SendQueueError(send));
            }
            Ok(())
        } else {
            Err(ClientError::NotListening)
        }
    }

    pub async fn send_seq(&mut self, buffer: &[u8], channel: u8) -> Result<(), ClientError> {
        if self.state.lock().await.is_available() {
            let mut send_q = self.send_queue.as_ref().unwrap().write().await;
            if let Err(send) = send_q
                .insert(
                    buffer.to_vec(),
                    Reliability::ReliableSeq,
                    false,
                    Some(channel),
                )
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
        &mut self,
        buffer: &[u8],
        reliability: Reliability,
        channel: u8,
    ) -> Result<(), ClientError> {
        if self.state.lock().await.is_available() {
            let mut send_q = self.send_queue.as_ref().unwrap().write().await;
            if let Err(send) = send_q
                .insert(buffer.to_vec(), reliability, false, Some(channel))
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

    pub async fn send_immediate(
        &mut self,
        buffer: &[u8],
        reliability: Reliability,
        channel: u8,
    ) -> Result<(), ClientError> {
        if self.state.lock().await.is_available() {
            let mut send_q = self.send_queue.as_ref().unwrap().write().await;
            if let Err(send) = send_q
                .insert(buffer.to_vec(), reliability, true, Some(channel))
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

    pub async fn flush_ack(&mut self) {
        let mut send_q = self.send_queue.as_ref().unwrap().write().await;
        let mut recv_q = self.recv_queue.lock().await;
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

    pub async fn recv(&mut self) -> Result<Vec<u8>, RecvError> {
        match self.internal_recv.recv().await {
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

    pub async fn ping(socket: Arc<UdpSocket>) -> Result<UnconnectedPong, ClientError> {
        let mut buf: [u8; 2048] = [0; 2048];
        let unconnected_ping = UnconnectedPing {
            timestamp: current_epoch(),
            magic: Magic::new(),
            client_id: rand::random::<i64>(),
        };

        if let Err(_) = socket
            .send(&Packet::from(unconnected_ping.clone()).parse().unwrap()[..])
            .await
        {
            rakrs_debug!(true, "[CLIENT] Failed to send ping packet!");
            return Err(ClientError::ServerOffline);
        }

        loop {
            rakrs_debug!(true, "[CLIENT] Waiting for pong packet...");
            if let Ok(recvd) = timeout(Duration::from_millis(10000), socket.recv(&mut buf)).await {
                match recvd {
                    Ok(l) => {
                        let packet = Packet::compose(&mut buf[..l], &mut 0).unwrap();
                        if packet.is_offline() {
                            match packet.get_offline() {
                                OfflinePacket::UnconnectedPong(pk) => {
                                    rakrs_debug!(true, "[CLIENT] Recieved pong packet!");
                                    return Ok(pk);
                                }
                                _ => {}
                            };
                        }
                    }
                    Err(_) => {
                        rakrs_debug!(true, "[CLIENT] Failed to recieve anything on netowrk channel, is there a sender?");
                        continue;
                    }
                }
            } else {
                rakrs_debug!(true, "[CLIENT] Ping Failed, server did not respond!");
                return Err(ClientError::ServerOffline);
            }
        }
    }

    async fn push_task(&mut self, task: JoinHandle<()>) {
        self.tasks.lock().await.push(task);
    }

    async fn init_recv_task(&mut self) -> Result<JoinHandle<()>, ClientError> {
        let net_recv = match self.network_recv {
            Some(ref n) => n.clone(),
            None => {
                rakrs_debug!("[CLIENT] (recv_task) Network recv channel is not initialized");
                return Err(ClientError::Killed);
            }
        };

        let send_queue = match self.send_queue {
            Some(ref s) => s.clone(),
            None => {
                rakrs_debug!("[CLIENT] (recv_task) Send queue is not initialized");
                return Err(ClientError::Killed);
            }
        };

        let recv_queue = self.recv_queue.clone();
        let internal_sender = self.internal_send.clone();
        let closed = self.closed.clone();
        let state = self.state.clone();
        let recv_time = self.recv_time.clone();

        let r = task::spawn(async move {
            loop {
                let pk_recv = net_recv.lock().await.recv().await;
                if closed.load(std::sync::atomic::Ordering::Relaxed) == true {
                    rakrs_debug!("[CLIENT] (recv_task) The internal network recieve channel has been killed.");
                    break;
                }

                #[cfg(feature = "async_std")]
                if let Err(_) = pk_recv {
                    rakrs_debug!(true, "[CLIENT] (recv_task) Failed to recieve anything on netowrk channel, is there a sender?");
                    continue;
                }

                #[cfg(feature = "async_tokio")]
                if let None = pk_recv {
                    rakrs_debug!(true, "[CLIENT] (recv_task)Failed to recieve anything on netowrk channel, is there a sender?");
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

                let mut buffer = pk_recv.unwrap();

                match buffer[0] {
                    0x80..=0x8d => {
                        if let Ok(frame_packet) = FramePacket::compose(&mut buffer[..], &mut 0) {
                            let mut recv_q = recv_queue.lock().await;
                            if let Err(_) = recv_q.insert(frame_packet) {
                                rakrs_debug!(
                                    true,
                                    "[CLIENT] Failed to push frame packet into send queue."
                                );
                            }

                            let buffers = recv_q.flush();

                            for mut pk_buf in buffers {
                                if let Ok(packet) = Packet::compose(&mut pk_buf[..], &mut 0) {
                                    if packet.is_online() {
                                        match packet.get_online() {
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
                                            }
                                            OnlinePacket::ConnectedPong(_) => {
                                                // todo: add ping time to client
                                                rakrs_debug!(
                                                    true,
                                                    "[CLIENT] Recieved pong packet!"
                                                );
                                            }
                                            OnlinePacket::Disconnect(_) => {
                                                rakrs_debug!(
                                                    true,
                                                    "[CLIENT] Recieved disconnect packet!"
                                                );
                                                break;
                                            }
                                            _ => {}
                                        };

                                        if let Err(_) = internal_sender.send(pk_buf).await {
                                            rakrs_debug!(true, "[CLIENT] Failed to send packet to internal recv channel. Is the client closed?");
                                        }
                                    } else {
                                        // we should never recieve an offline packet after the handshake
                                        rakrs_debug!("[CLIENT] Recieved offline packet after handshake! In future versions this will kill the client.");
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
                    NACK => {
                        if let Ok(nack) = Ack::compose(&mut buffer[..], &mut 0) {
                            let mut send_q = send_queue.write().await;
                            let to_resend = send_q.nack(nack);

                            if to_resend.len() > 0 {
                                for ack_packet in to_resend {
                                    if let Ok(buffer) = ack_packet.parse() {
                                        if let Err(_) = send_q
                                            .insert(buffer, Reliability::Unreliable, true, Some(0))
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
                        if let Ok(ack) = Ack::compose(&mut buffer[..], &mut 0) {
                            let mut send_q = send_queue.write().await;
                            send_q.ack(ack.clone());

                            drop(send_q);

                            recv_queue.lock().await.ack(ack);
                        }
                    }
                    _ => {
                        // we don't know what this is, so we're going to send it to the user, maybe
                        // this is a custom packet
                        if let Err(_) = internal_sender.send(buffer).await {
                            rakrs_debug!(true, "[CLIENT] Failed to send packet to internal recv channel. Is the client closed?");
                        }
                    }
                }
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

                if closer.load(std::sync::atomic::Ordering::Relaxed) == true {
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

impl Drop for Client {
    fn drop(&mut self) {
        self.closed
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
