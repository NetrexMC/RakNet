use rakrs::Motd;
use rakrs::RakEvent;
use rakrs::RakNetServer;
use rakrs::RakResult;
use rakrs::start;

#[tokio::main]
pub async fn main() {
    let mut server = RakNetServer::new(String::from("0.0.0.0:19132"));
    let mut motd = Motd {
        name: "Netrex Raknet".to_owned(),
        protocol: 190,
        player_count: 0,
        player_max: 10000,
        gamemode: "creative".to_owned(),
        version: "1.18.9".to_owned(),
        server_id: 2747994720109207713 as i64,
    };
    let channel = netrex_events::Channel::<RakEvent, RakResult>::new();
    let mut unknown = 0;
    let mut listener = |event, result| {
        match event {
            RakEvent::ConnectionCreated(_) => {
                println!("Client connected");
            }
            RakEvent::Disconnect(_, _) => {
                println!("Client disconnected");
            }
            _ => {
                unknown += 1;
                println!("Unknown events: {}", unknown);
            }
        };
        None
    };
    channel.receive(&mut listener);

    println!("Hi I am running concurrently.");

    start(server, channel).await;
}