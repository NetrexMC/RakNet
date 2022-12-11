#[derive(Debug, Clone)]
pub enum ServerError {
    AddrBindErr,
    ServerRunning,
}
