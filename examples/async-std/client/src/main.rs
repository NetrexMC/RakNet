use rakrs::client::{Client, DEFAULT_MTU};

#[async_std::main]
async fn main() {
    let mut client = Client::new(11, DEFAULT_MTU);

    client.connect("135.148.137.229:19132").await.unwrap();

    loop {
        let pk = client.recv().await.unwrap();
        println!("Received packet: {:?}", pk);
    }
}