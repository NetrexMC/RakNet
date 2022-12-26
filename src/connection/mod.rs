pub mod queue;
pub mod state;

use std::{net::SocketAddr, sync::Arc, time::SystemTime};

use binary_utils::Streamable;
use tokio::{
    net::UdpSocket,
    sync::{mpsc, oneshot, Mutex, Notify, RwLock, Semaphore},
};

use crate::{
    error::connection::ConnectionError,
    protocol::packet::Packet,
    rakrs_debug,
    server::{
        event::{ServerEvent, ServerEventResponse},
        raknet_start,
    },
    util::to_address_token,
};

use self::queue::{RecvQueue, SendQueue};

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
            recv_time: raknet_start(),
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
    /// The queue used to send packets back to the connection.
    pub(crate) send_queue: Arc<RwLock<SendQueue>>,
    /// The queue used to recieve packets, this is read from by the server.
    /// This is only used internally.
    pub(crate) recv_queue: Arc<Mutex<RecvQueue>>,
    pub(crate) state: state::ConnectionState,
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
            state: state::ConnectionState::Unidentified,
            disconnect: Arc::new(Semaphore::new(0)),
            recv_time: Arc::new(Mutex::new(SystemTime::now())),
        };

        c.init_tick(socket).await;
        c.init_net_recv(socket, net, net_sender).await;

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
        let state = self.state;
        let address = self.address;

        tokio::spawn(async move {
            loop {
                if disconnect.is_closed() {
                    rakrs_debug!(
                        "[{}] Network reciever task disbanding!",
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
                    rakrs_debug!(
                        true,
                        "[{}] Unknown RakNet packet recieved.",
                        to_address_token(address)
                    );
                }
            }
        });
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
