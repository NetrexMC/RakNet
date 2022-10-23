pub mod queue;

use std::{net::SocketAddr, sync::Arc};

use tokio::sync::RwLock;

use self::queue::SendQueue;

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
}