//! Client errors are errors that can occur when using the [`Client`](crate::client::Client) api.
use crate::{client::handshake::HandshakeStatus, connection::queue::SendQueueError};

/// These are errors that can occur when using the [`Client`](crate::client::Client) api.
/// These are returned for a variety of reasons, but is commonly used to indicate
/// that something went wrong, and you should either clean up or retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ClientError {
    /// The client is already connected to the peer on this address.
    AddrBindErr,
    /// The client is already connected a peer.
    AlreadyOnline,
    /// The client is offline and can not send packets.
    NotListening,
    /// The client is unable to connect to the peer.
    Unavailable,
    /// The client is unable to connect to the peer because the peer is using a different protocol version.
    IncompatibleProtocolVersion,
    /// The client has been closed.
    Killed,
    /// The client has been closed, and can not be used again.
    Reset,
    /// The client is unable to connect to the peer because the peer is offline.
    ServerOffline,
    /// The client failed to process a packet you sent.
    SendQueueError(SendQueueError),
    /// The client errored during handshake.
    HandshakeError(HandshakeStatus),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ClientError::AddrBindErr => "Failed to bind to address",
                ClientError::AlreadyOnline => "Already online",
                ClientError::NotListening => "Not listening",
                ClientError::Unavailable => "Unavailable",
                ClientError::IncompatibleProtocolVersion => "Incompatible protocol version",
                ClientError::Killed => "Killed",
                ClientError::Reset => "Reset",
                ClientError::ServerOffline => "Server offline",
                ClientError::SendQueueError(e) => return e.fmt(f),
                ClientError::HandshakeError(e) => return e.fmt(f),
            }
        )
    }
}

impl std::error::Error for ClientError {}
