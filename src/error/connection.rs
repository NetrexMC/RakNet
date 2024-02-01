//! # Connection Error
//! These error types are used when an error occurs within the [`Connection`].
//!
//! [`Connection`]: crate::connection::Connection
/// The error type for the [`Connection`].
/// These are lesser known errors that can occur within the connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConnectionError {
    /// The connection has been closed.
    Closed,
    /// The connection has been closed by the peer.
    EventDispatchError,
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ConnectionError::Closed => "Connection closed",
                ConnectionError::EventDispatchError => "Event dispatch error",
            }
        )
    }
}

impl std::error::Error for ConnectionError {}
