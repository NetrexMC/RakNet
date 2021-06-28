use std::net::SocketAddr;

pub trait ConnectionAPI {

}

pub struct Connection {
     address: SocketAddr,
     // queue: PacketQueue<>
}