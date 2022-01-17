#![feature(cursor_remaining)]
extern crate binary_utils;

/// A unique identifier recoginzing the client as offline.
pub const MAGIC: [u8; 16] = [
    0x00, 0xff, 0xff, 0x0, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78,
];

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
