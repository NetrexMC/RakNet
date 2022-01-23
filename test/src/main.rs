use rakrs::start;
use rakrs::RakEvent;
use rakrs::RakNetServer;
use rakrs::RakResult;
use tokio::runtime::Runtime;

#[tokio::main]
pub async fn main() {
    let server = RakNetServer::new(String::from("0.0.0.0:19132"));
    let channel = netrex_events::Channel::<RakEvent, RakResult>::new();
    let mut unknown = 0;
    let mut listener = |event, _| {
        match event {
            RakEvent::ConnectionCreated(address) => {
                println!("[RakNet] [{}] Client connected", address);
            }
            RakEvent::Disconnect(address, reason) => {
                println!(
                    "[RakNet] [{}] Client disconnected due to: {}",
                    address, reason
                );
            }
            RakEvent::Motd(address, mut motd) => {
                // println!("[RakNet] [{}] Client requested motd: {:?}", address, motd);
                motd.name = String::from("RakRS v2");
                return Some(RakResult::Motd(motd));
            }
            RakEvent::GamePacket(address, packet) => {
                println!("[RakNet] [{}] Client sent packet: {:?}", address, packet);
                return None;
            }
            _ => {
                unknown += 1;
                println!("Unknown events: {}", unknown);
            }
        };
        None
    };
    channel.receive(&mut listener);

    let v = start(server, channel).await;
    v.0.await;
}