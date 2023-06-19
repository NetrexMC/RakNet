use rak_rs::client::{Client, DEFAULT_MTU};
use std::{net::ToSocketAddrs, vec};

#[async_std::main]
async fn main() {
    let mut client = Client::new(10, DEFAULT_MTU);
    let mut addr = "zeqa.net:19132".to_socket_addrs().unwrap();
    client.connect(addr.next().unwrap()).await.unwrap();

    loop {
        let pk = client.recv().await.unwrap();
        println!("Received packet: {:?}", pk);
        client.send_ord(&vec![ 254, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00 ], 1).await.unwrap();
    }
}