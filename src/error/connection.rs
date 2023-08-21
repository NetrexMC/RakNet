//! # Connection Error
//! These error types are used when an error occurs within the [`Connection`].
//!
//! [`Connection`]: crate::connection::Connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConnectionError {
    Closed,
    EventDispatchError,
}
