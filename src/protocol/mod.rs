pub(crate) mod ack;
pub(crate) mod frame;
pub(crate) mod magic;
pub mod mcpe;
pub mod packet;
pub mod reliability;

pub use magic::*;

pub const MAX_FRAGS: u32 = 1024;
pub const MAX_ORD_CHANS: u8 = 32;
