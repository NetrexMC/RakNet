#![feature(cursor_remaining)]
extern crate binary_utils;

pub mod ack;
pub mod conn;
pub mod frame;
pub mod protocol;
pub mod server;
pub mod util;

pub const MAGIC: [u8; 16] = [
    0x00, 0xff, 0xff, 0x0, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78,
];
pub const SERVER_ID: i64 = 2747994720109207718; //rand::random::<i64>();
pub const USE_SECURITY: bool = false;

pub use self::{frame::*, protocol::*, server::*, util::*};