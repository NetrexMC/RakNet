//! # rak-rs
//!
//! A fully functional RakNet implementation in pure rust, asynchronously driven.
//!
//! ## Getting Started
//!
//! RakNet (rak-rs) is available on [crates.io](https://crates.io/crates/rak-rs), to use it, add the following to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! rak-rs = "0.3.3"
//! ```
//!
//! ## Features
//!
//! This RakNet implementation comes with 3 primary features, `async_std`, `async_tokio` and `mcpe`.  However, by default, only `async_std` is enabled, and `mcpe` requires you to modify your `Cargo.toml`.
//!
//! If you wish to use these features, add them to your `Cargo.toml` as seen below:
//!
//! ```toml
//! [dependencies]
//! rak-rs = { version = "0.3.3", default-features = false, features = [ "async_tokio", "mcpe" ] }
//! ```
//!
//!
//!
//! rak-rs also provides the following modules:
//!
//! - [`rak_rs::client`](crate::client) - A client implementation of RakNet, allowing you to connect to a RakNet server.
//! - [`rak_rs::connection`](crate::connection) - A bare-bones implementation of a Raknet peer, this is mainly used for types.
//! - [`rak_rs::error`](crate::error) - A module with errors that both the Client and Server can respond with.
//! - [`rak_rs::protocol`](crate::protocol) - A lower level implementation of RakNet, responsible for encoding and decoding packets.
//! - [`rak_rs::server`](crate::server) - The base server implementation of RakNet.
//! - [`rak_rs::util`](crate::util)  - General utilities used within `rak-rs`.
//!
//! # Client
//!
//! The `client` module provides a way for you to interact with RakNet servers with code.
//!
//! **Example:**
//!
//! ```ignore
//! use rak_rs::client::{Client, DEFAULT_MTU};
//! use std::net::ToSocketAddrs;
//!
//! #[async_std::main]
//! async fn main() {
//!     let version: u8 = 10;
//!     let mut client = Client::new(version, DEFAULT_MTU);
//!
//!     client.connect("my_server.net:19132").await.unwrap();
//!
//!     // receive packets
//!     loop {
//!         let packet = client.recv().await.unwrap();
//!
//!         println!("Received a packet! {:?}", packet);
//!
//!         client.send_ord(vec![254, 0, 1, 1], Some(1));
//!     }
//! }
//!
//! ```
//!
//! # Server
//!
//! A RakNet server implementation in pure rust.
//!
//! **Example:**
//!
//! ```ignore
//! use rakrs::connection::Connection;
//! use rakrs::Listener;
//! use rakrs::
//!
//! #[async_std::main]
//! async fn main() {
//!     let mut server = Listener::bind("0.0.0.0:19132").await.unwrap();
//!     server.start().await.unwrap();
//!
//!     loop {
//!         let conn = server.accept().await;
//!         async_std::task::spawn(handle(conn.unwrap()));
//!     }
//! }
//!
//! async fn handle(mut conn: Connection) {
//!     loop {
//!         // keeping the connection alive
//!         if conn.is_closed() {
//!             println!("Connection closed!");
//!             break;
//!         }
//!         if let Ok(pk) = conn.recv().await {
//!             println!("Got a connection packet {:?} ", pk);
//!         }
//!     }
//! }
//! ```
/// A client implementation of RakNet, allowing you to connect to a RakNet server.
pub mod client;
/// The connection implementation of RakNet, allowing you to send and receive packets.
/// This is barebones, and you should use the client or server implementations instead, this is mainly
/// used internally.
pub mod connection;
/// The error implementation of RakNet, allowing you to handle errors.
pub mod error;
/// The packet implementation of RakNet.
/// This is a lower level implementation responsible for serializing and deserializing packets.
pub mod protocol;
/// The server implementation of RakNet, allowing you to create a RakNet server.
pub mod server;
/// Utilties for RakNet, like epoch time.
pub mod util;

pub use protocol::mcpe::{self, motd::Motd};
pub use server::Listener;

/// An internal module for notifying the connection of state updates.
pub(crate) mod notify;
