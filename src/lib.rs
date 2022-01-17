#![feature(cursor_remaining)]
extern crate binary_utils;

/// Home of the RakNet protocol.
/// This contains some generic handling for the protocol.
/// If you're looking for mcpe specific handling you need
/// to enable the `mcpe` feature.
pub mod protocol;

/// Raknet sessions.
/// These should be used to communicate with other players. (on the server)
/// A session is a "connection" to a player. It serves as the interface for
/// communicating with a client.
pub mod session;