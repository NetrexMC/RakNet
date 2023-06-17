#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConnectionError {
    Closed,
    EventDispatchError,
}
