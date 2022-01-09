use rakrs::Motd;
use rakrs::RakEvent;
use rakrs::RakNetServer;
use rakrs::RakResult;
use rakrs::start;
use tokio::runtime::Runtime;


#[test]
pub fn test_boot() {
    let server = RakNetServer::new(String::from("0.0.0.0:19132"));
    let motd = Motd::new(server.server_guid);
    let channel = netrex_events::Channel::<RakEvent, RakResult>::new();
    let mut unknown = 0;
    let mut listener = |event, _| {
        match event {
            RakEvent::ConnectionCreated(address) => {
                println!("[RakNet] [{}] Client connected", address);
            }
            RakEvent::Disconnect(address, reason) => {
                println!("[RakNet] [{}] Client disconnected due to: {}", address, reason);
            }
            _ => {
                unknown += 1;
                println!("Unknown events: {}", unknown);
            }
        };
        None
    };
    channel.receive(&mut listener);

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();
    handle.block_on(async move {
        start(server, channel).await;
    });
}