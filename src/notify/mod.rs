#[cfg(feature = "async_std")]
mod async_std;

#[cfg(feature = "tokio")]
mod tokio;

#[cfg(feature = "async_std")]
pub use async_std::Notify;

#[cfg(feature = "tokio")]
pub use tokio::Notify;
