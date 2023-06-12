use rakrs::client::{Client, DEFAULT_MTU};

#[async_std::main]
async fn main() {
    let mut client = Client::new(8, DEFAULT_MTU);

    client.connect("127.0.0.1:19132").await.unwrap();

    loop {
        let pk = client.recv().await.unwrap();
        println!("Received packet: {:?}", pk);
    }
}