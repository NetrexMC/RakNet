#[derive(Debug, Clone)]
pub enum ServerError {
    AddrBindErr,
    AlreadyOnline,
    NotListening,
    Killed,
    Reset,
}
