use rakrs::raknet_start;
use rakrs::Motd;
use rakrs::RakNetEvent;
use rakrs::RakNetServer;
use rakrs::RakResult;

#[test]
pub fn test_server() {
    let mut server = RakNetServer::new(String::from("0.0.0.0:19132"));
    server.set_motd(Motd {
        name: "Netrex Raknet".to_owned(),
        protocol: 190,
        player_count: 0,
        player_max: 10000,
        gamemode: "creative".to_owned(),
        version: "1.18.9".to_owned(),
        server_id: 2747994720109207718 as i64,
    });
    let channel = netrex_events::Channel::<RakNetEvent, RakResult>::new();
    let mut unknown = 0;
    let mut listener = |event, result| {
        match event {
            RakNetEvent::ConnectionCreated(_) => {
                println!("Client connected");
            }
            RakNetEvent::Disconnect(_, _) => {
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
    let server = raknet_start!(server, channel);

    println!("Hi I am running concurrently.");
}
