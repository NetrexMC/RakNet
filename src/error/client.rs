#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ClientError {
    AddrBindErr,
    AlreadyOnline,
    NotListening,
    IncompatibleProtocolVersion,
    Killed,
    Reset,
}