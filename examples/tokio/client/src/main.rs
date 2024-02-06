use rak_rs::{
    client::{Client, DEFAULT_MTU}
};

#[tokio::main]
async fn main() {
    let mut client = Client::new(11, DEFAULT_MTU)
        .with_timeout(20)
        .with_handshake_timeout(3);

    for _ in 0..3 {
        if let Err(e) = client.connect("zeqa.net:19132").await {
            println!("Failed to connect: {}, trying again...", e);
        } else {
            break;
        }
    }

    if !client.is_connected().await {
        println!("Failed to connect to server after 3 attempts. Exiting...");
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
