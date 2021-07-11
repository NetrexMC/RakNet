use std::net::SocketAddr;
use std::collections::VecDeque;
use binary_utils::*;

pub trait ConnectionAPI {
     fn receive(&self, stream: &BinaryStream);
}

#[derive(Clone)]
pub struct Connection {
     // read by raknet
     pub send_queue: VecDeque<BinaryStream>,
     address: SocketAddr,
}

impl Connection {
     pub fn new(address: SocketAddr) -> Self {
          Self {
               send_queue: VecDeque::new(),
               address
          }
     }
}

impl ConnectionAPI for Connection {
     fn receive(&self, stream: &BinaryStream) {
          // dummy implementation, not expected to be used here, yet.
          println!("Got a buffer! {:?}", stream);
     }
}