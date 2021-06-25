use std::{ thread, io, net::SocketAddr, sync::Arc, sync::mpsc };
use tokio::net::UdpSocket;
use rand::random;
use async_trait::async_trait;
use super::conn::Connection;

#[async_trait]
pub trait IRakServer {
     fn new(address: String, version: u8) -> Self;

     async fn start(&mut self) -> io::Result<()>;
}

pub struct RakServer {
     socket: Option<Arc<UdpSocket>>,
     address: String,
     version: u8,
     connections: Vec<Connection>
}

#[async_trait]
impl IRakServer for RakServer {
     fn new(address: String, version: u8) -> Self {
          Self {
               socket: None,
               version,
               address,
               connections: Vec::new()
          }
     }

     async fn start(&mut self) -> io::Result<()> {
          let socket = UdpSocket::bind(self.address.as_str()).await?;
          self.socket = Some(Arc::new(socket));

          let mut buf: [u8; 1024] = [0; 1024];
          loop {
               let len = self.socket.unwrap().recv(&mut buf).await?;
          }
     }
}