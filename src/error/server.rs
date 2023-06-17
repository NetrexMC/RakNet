#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ServerError {
    AddrBindErr,
    AlreadyOnline,
    NotListening,
    Killed,
    Reset,
}
