use crate::connection::queue::SendQueueError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ClientError {
    AddrBindErr,
    AlreadyOnline,
    NotListening,
    Unavailable,
    IncompatibleProtocolVersion,
    Killed,
    Reset,
    SendQueueError(SendQueueError),
}
