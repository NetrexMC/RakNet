pub mod queue;
pub mod state;

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use tokio::sync::RwLock;

use self::queue::{RecvQueue, SendQueue};

#[derive(Debug, Clone, Copy)]
pub struct ConnMeta {
    /// This is important, and is stored within the server itself
    /// This value is 0 until the connection state is `Connecting`
    pub mtu_size: u16,
    /// The state of the connection
    pub state: state::ConnState
}

/// This struct is utilized internally and represented
/// as per each "connection" or "socket" to the server.
/// Each Connection has it's own Reference pointer to a
/// socket dedicated to this connection.
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
}
