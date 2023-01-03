pub mod controller;
/// Necessary queues for the connection.
pub mod queue;
pub mod state;

use std::{net::SocketAddr, sync::Arc, time::SystemTime};

use binary_utils::Streamable;
use tokio::{
    net::UdpSocket,
    sync::{
        mpsc::{self, Sender},
        oneshot, Mutex, Notify, RwLock, Semaphore,
    },
};

use crate::{
    error::connection::ConnectionError,
    protocol::{
        ack::{Ack, Ackable, ACK, NACK},
        frame::FramePacket,
        packet::{online::OnlinePacket, Packet},
    },
    rakrs_debug,
    server::{
        current_epoch,
        event::{ServerEvent, ServerEventResponse},
    },
    util::to_address_token,
};

use self::{
    controller::window::ReliableWindow,
    queue::{RecvQueue, SendQueue},
    state::ConnectionState,
};

pub(crate) type ConnDispatcher =
    mpsc::Receiver<(ServerEvent, oneshot::Sender<ServerEventResponse>)>;
pub(crate) type ConnEvtChan = Arc<Mutex<ConnDispatcher>>;
pub(crate) type ConnNetChan = Arc<Mutex<mpsc::Receiver<Vec<u8>>>>;

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

/// This struct is utilized internally and represented
/// as per each "connection" or "socket" to the server.
/// This is **NOT** a struct that supports connecting TO
/// a RakNet instance, but rather a struct that HOLDS a
/// inbound connection connecting to a `Listener` instance.
///
/// Each Connection has it's own channel for recieving
/// buffers that come from this address.
pub struct Connection {
    /// The address of the connection
    /// This is internally tokenized by rak-rs
    pub address: SocketAddr,
    pub(crate) state: Arc<Mutex<ConnectionState>>,
    /// The queue used to send packets back to the connection.
    send_queue: Arc<RwLock<SendQueue>>,
    /// The queue used to recieve packets, this is read from by the server.
    /// This is only used internally.
    recv_queue: Arc<Mutex<RecvQueue>>,
    /// The network channel, this is where the connection will be recieving it's packets.
    /// This is interfaced to provide the api for `Connection::recv()`
    internal_net_recv: ConnNetChan,
    /// The event IO to communicate with the listener.
    /// This is responsible for refreshing the Motd and any other overhead like,
    /// raknet voice channels or plugins, however these have not been implemented yet.
    dispatch: ConnEvtChan,
    /// A notifier for when the connection should close.
    /// This is used for absolute cleanup withtin the connection
    disconnect: Arc<Semaphore>,
    /// The last time a packet was recieved. This is used to keep the connection from
    /// being in memory longer than it should be.
    recv_time: Arc<Mutex<SystemTime>>,
}

impl Connection {
    /// Initializes a new Connection instance.
    pub async fn new(
        address: SocketAddr,
        socket: &Arc<UdpSocket>,
        net: mpsc::Receiver<Vec<u8>>,
        dispatch: ConnDispatcher,
    ) -> Self {
        let (net_sender, net_receiver) = mpsc::channel::<Vec<u8>>(100);
        let c = Self {
            address,
            send_queue: Arc::new(RwLock::new(SendQueue::new())),
            recv_queue: Arc::new(Mutex::new(RecvQueue::new())),
            internal_net_recv: Arc::new(Mutex::new(net_receiver)),
            dispatch: Arc::new(Mutex::new(dispatch)),
            state: Arc::new(Mutex::new(ConnectionState::Unidentified)),
            disconnect: Arc::new(Semaphore::new(0)),
            recv_time: Arc::new(Mutex::new(SystemTime::now())),
        };

        c.init_tick(socket).await;
        c.init_net_recv(socket, net, net_sender).await;

        // todo finish the send queue
        // todo finish the ticking function
        // todo add function for user to accept and send packets!

        return c;
    }

    /// Initializes the client ticking process!
    pub async fn init_tick(&self, socket: &Arc<UdpSocket>) {
        // initialize the event io
        // we initialize the ticking function here, it's purpose is to update the state of the current connection
        // while handling throttle
    }

    /// This function initializes the raw internal packet handling task!
    ///
    pub async fn init_net_recv(
        &self,
        socket: &Arc<UdpSocket>,
        mut net: mpsc::Receiver<Vec<u8>>,
        sender: mpsc::Sender<Vec<u8>>,
    ) {
        let recv_time = self.recv_time.clone();
        let recv_q = self.recv_queue.clone();
        let send_q = self.send_queue.clone();
        let disconnect = self.disconnect.clone();
        let state = self.state.clone();
        let address = self.address;

        tokio::spawn(async move {
            loop {
                if disconnect.is_closed() {
                    rakrs_debug!(
                        true,
                        "[{}] Recv task has been closed!",
                        to_address_token(address)
                    );
                    break;
                }

                if let Some(payload) = net.recv().await {
                    // We've recieved a payload!
                    drop(*recv_time.lock().await = SystemTime::now());

                    // Validate this packet
                    // this is a raw buffer!
                    // There's a few things that need to be done here.
                    // 1. Validate the type of packet
                    // 2. Determine how the packet should be processed
                    // 3. If the packet is assembled and should be sent to the client
                    //    send it to the communication channel using `sender`

                    let id = payload[0];
                    match id {
                        // This is a frame packet.
                        // This packet will be handled by the recv_queue
                        0x80..=0x8d => {
                            if let Ok(pk) = FramePacket::compose(&payload[..], &mut 0) {
                                let mut rq = recv_q.lock().await;

                                if let Ok(_) = rq.insert(pk) {};

                                let buffers = rq.flush();

                                for buffer in buffers {
                                    let res = Connection::process_packet(
                                        &buffer, &address, &sender, &send_q, &state,
                                    )
                                    .await;
                                    if let Ok(v) = res {
                                        if v == true {
                                            // DISCONNECT
                                            disconnect.close();
                                            break;
                                        }
                                    }
                                    if let Err(_) = res {
                                        rakrs_debug!(
                                            true,
                                            "[{}] Failed to process packet!",
                                            to_address_token(address)
                                        );
                                    };
                                }

                                drop(rq);
                            }
                        }
                        NACK => {
                            // Validate this is a nack packet
                            if let Ok(nack) = Ack::compose(&payload[..], &mut 0) {
                                // The client acknowledges it did not recieve these packets
                                // We should resend them.
                                let mut sq = send_q.write().await;
                                let resend = sq.nack(nack);
                                for packet in resend {
                                    if let Err(_) = sender.send(packet).await {
                                        rakrs_debug!(
                                            true,
                                            "[{}] Failed to send packet to client!",
                                            to_address_token(address)
                                        );
                                    }
                                }
                            }
                        }
                        ACK => {
                            // first lets validate this is an ack packet
                            if let Ok(ack) = Ack::compose(&payload[..], &mut 0) {
                                // The client acknowledges it recieved these packets
                                // We should remove them from the queue.
                                let mut sq = send_q.write().await;
                                sq.ack(ack);
                            }
                        }
                        _ => {
                            rakrs_debug!(
                                true,
                                "[{}] Unknown RakNet packet recieved (Or packet is sent out of scope).",
                                to_address_token(address)
                            );
                        }
                    };
                }
            }
        });
    }

    pub async fn process_packet(
        buffer: &Vec<u8>,
        address: &SocketAddr,
        sender: &Sender<Vec<u8>>,
        send_q: &Arc<RwLock<SendQueue>>,
        state: &Arc<Mutex<ConnectionState>>,
    ) -> Result<bool, ()> {
        if let Ok(packet) = Packet::compose(buffer, &mut 0) {
            if packet.is_online() {
                return match packet.get_online() {
                    OnlinePacket::ConnectedPing(pk) => {
                        let response = ConnectedPong {
                            ping_time: pk.time,
                            pong_time: SystemTime::now()
                                .duration_since(connection.start_time)
                                .unwrap()
                                .as_millis() as i64,
                        };
                        Ok(true)
                    }
                    OnlinePacket::ConnectionRequest(pk) => {
                        let response = ConnectionAccept {
                            system_index: 0,
                            client_address: from_address_token(connection.address.clone()),
                            internal_id: SocketAddr::new(
                                IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)),
                                19132,
                            ),
                            request_time: pk.time,
                            timestamp: SystemTime::now()
                                .duration_since(connection.start_time)
                                .unwrap()
                                .as_millis() as i64,
                        };
                        Ok(true)
                    }
                    OnlinePacket::Disconnect(_) => {
                        // Disconnect the client immediately.
                        connection.disconnect("Client disconnected.", false);
                        Ok(false)
                    }
                    OnlinePacket::NewConnection(_) => {
                        connection.state = ConnectionState::Connected;
                        Ok(true)
                    }
                    _ => {
                        sender.send(buffer.clone()).await.unwrap();
                        Ok(true)
                    }
                };
            } else {
                *state.lock().await = ConnectionState::Disconnecting;
                rakrs_debug!(
                    true,
                    "[{}] Invalid protocol! Disconnecting client!",
                    to_address_token(*address)
                );
                return Err(());
            }
        }
        Err(())
    }

    /// Handle a RakNet Event. These are sent as they happen.
    ///
    /// EG:
    /// ```ignore
    /// let conn: Connection = Connection::new();
    ///
    /// while let Some((event, responder)) = conn.recv_ev {
    ///     match event {
    ///         ServerEvent::SetMtuSize(mtu) => {
    ///             println!("client updated mtu!");
    ///             responder.send(ServerEventResponse::Acknowledge);
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn recv_ev(
        &self,
    ) -> Result<(ServerEvent, oneshot::Sender<ServerEventResponse>), ConnectionError> {
        match self.dispatch.lock().await.recv().await {
            Some((server_event, event_responder)) => {
                return Ok((server_event, event_responder));
            }
            None => {
                if self.disconnect.is_closed() {
                    return Err(ConnectionError::Closed);
                }
                return Err(ConnectionError::EventDispatchError);
            }
        }
    }

    /// Initializes the client tick.
    pub async fn tick(&mut self) {
        let sendq = self.send_queue.write().await;
        // sendq.tick().await;
    }
}
