pub(crate) mod ack;
pub(crate) mod frame;
pub(crate) mod magic;
pub mod mcpe;
pub mod packet;
pub mod reliability;

pub use magic::*;

pub const MAX_FRAGS: u32 = 1024;
pub const MAX_ORD_CHANS: u8 = 32;

// IP Header + UDP Header + RakNet Header + RakNet Frame Packet Header (MAX)
pub const RAKNET_HEADER_FRAME_OVERHEAD: u16 = 20 + 8 + 8 + 4 + 20;
// IP Header + UDP Header + RakNet Header
pub const RAKNET_HEADER_OVERHEAD: u16 = 20 + 8 + 8;
