use std::{sync::Arc, net::SocketAddr};

use tokio::net::UdpSocket;

pub struct Listener {
    /// The current socket.
    sock: Option<Arc<UdpSocket>>,
    /// Whether or not to use minecraft specific protocol.
    /// This only effects the `Pong` packet where motd is sent.
    mcpe: bool,
    /// If mcpe is true, this is the default MOTD, this is
    /// the default MOTD to send to the client.
    motd: String,
}

impl Listener {
    pub async fn bind(address: SocketAddr) {
        
    }
}