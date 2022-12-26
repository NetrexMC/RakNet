#[derive(Debug, Clone, Copy)]
pub enum ConnectionError {
    Closed,
    EventDispatchError,
}
