mod packet;
/// Packet Utilities
pub use packet::*;

#[cfg(feature = "mcpe")]
pub mod mcpe;

/// The protocol that is used when a client is considered "Online".
/// Sessions in this state are usually: Connected or Connecting
pub mod online;

/// The protocol that is used when a client is considered "Unidentified".
/// This module is used for Pong and connection requests.
pub mod offline;
