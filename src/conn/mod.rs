pub mod queue;
pub mod state;

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use tokio::{
    net::UdpSocket,
    sync::{mpsc, oneshot, RwLock},
};

use crate::server::{
    event::{ServerEvent, ServerEventResponse},
    raknet_start,
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
pub struct Conn {
    /// The address of the connection
    /// This is internally tokenized by rak-rs
    pub address: SocketAddr,

    /// The queue used to send packets back to the connection.
    pub(crate) send_queue: Arc<RwLock<SendQueue>>,

    /// The queue used to recieve packets, this is read from by the server.
    /// This is only used internally.
    pub(crate) recv_queue: Arc<Mutex<RecvQueue>>,

    pub(crate) state: state::ConnState,

    /// The network channel, this is where the connection will be recieving it's packets.
    /// If the channel is dropped, the connection is expected to drop as well, this behavior
    /// has not been implemented yet.
    net: ConnNetChan,
    /// The event IO to communicate with the listener.
    /// This is responsible for refreshing the Motd and any other overhead like,
    /// raknet voice channels or plugins, however these have not been implemented yet.
    dispatch: ConnEvtChan,
}

impl Conn {
    /// Initializes a new Connection instance.
    pub fn new(
        address: SocketAddr,
        socket: &Arc<UdpSocket>,
        net: mpsc::Receiver<Vec<u8>>,
        dispatch: ConnDispatcher,
    ) -> Self {
    }
}
