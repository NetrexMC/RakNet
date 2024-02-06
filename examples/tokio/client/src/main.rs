use rak_rs::{
    client::{Client, DEFAULT_MTU},
    protocol::reliability::Reliability,
};

#[tokio::main]
async fn main() {
    let mut client = Client::new(10, DEFAULT_MTU);

    if let Err(e) = client.connect("zeqa.net:19132").await {
        println!("Failed to connect: {}", e);
        return;
    }

    client
        .send_ord(&[254, 6, 193, 1, 0, 0, 2, 118], 0)
        .await
        .unwrap();

    loop {
        let packet = client.recv().await.unwrap();
        if packet[0] == 0xfe {
            println!("Received (Game): {:?}", packet);
        } else {
            println!("Received: {:?}", packet);
        }
    }
}
