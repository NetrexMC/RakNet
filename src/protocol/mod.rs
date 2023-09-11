//! Protocol implementation for RakNet.
//! this module contains all of the necessary packets to communicate with RakNet servers.
//!
//! ## Example
//! Please note this example is not a copy-paste example, but rather a snippet of code
//! that shows how to use the [`protocol`] module.
//!
//! ```rust ignore
//! use rakrs::protocol::packet::RakPacket;
//! use binary_util::BinaryIo;
//!
//! fn decode_packet(packet: &[u8]) {
//!     let packet = RakPacket::read_from_slice(packet).unwrap();
//!
//!     match packet {
//!         RakPacket::Offline(packet) => match packet {
//!             OfflinePacket::UnconnectedPing(packet) => {
//!                 println!("Received a ping packet! {:?}", packet);
//!             },
//!             _ => {}
//!         },
//!         _ => {}
//!     }
//! };
//! ```
/// This is an internal module that contains the logic to implement the Ack system within
/// RakNet.
pub(crate) mod ack;
/// This is an internal module that contains the logic to implement the frame system within
/// RakNet. This is also called the "Datagram" or "Encapsulated" packet in different implementations.
///
/// You can find the original implementation from RakNet [here](https://github.com/facebookarchive/RakNet/blob/1a169895a900c9fc4841c556e16514182b75faf8/Source/ReliabilityLayer.cpp#L110-L231)
// pub(crate) mod frame;
pub mod frame;
/// This is the constant added to all offline packets to identify them as RakNet packets.
pub(crate) mod magic;
/// This module contains the MCPE specific packets that are used within RakNet, this is guarded
/// under the `mcpe` feature.
///
/// To enable this feature, add the following to your `Cargo.toml`:
/// ```toml
/// [dependencies]
/// rak-rs = { version = "0.1.0", features = [ "mcpe" ] }
/// ```
pub mod mcpe;
pub mod packet;
pub mod reliability;

pub use magic::*;

/// The maximum amount of fragments that can be sent within a single frame.
/// This constant is used to prevent a client from sending too many fragments,
/// or from bad actors from sending too many fragments.
pub const MAX_FRAGS: u32 = 1024;
/// The maximum amount of channels that can be used on a single connection.
/// This is a raknet limitation, and is not configurable.
pub const MAX_ORD_CHANS: u8 = 32;

/// IP Header + UDP Header + RakNet Header + RakNet Frame Packet Header (MAX)
pub const RAKNET_HEADER_FRAME_OVERHEAD: u16 = 20 + 8 + 8 + 4 + 20;
/// IP Header + UDP Header + RakNet Header
pub const RAKNET_HEADER_OVERHEAD: u16 = 20 + 8 + 8;

/// The maximum possible amount of bytes that can be sent within a single frame.
pub const MTU_MAX: u16 = 2400;
/// The minimum possible amount of bytes that can be sent within a single frame.
pub const MTU_MIN: u16 = 400;
