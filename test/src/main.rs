use rakrs::RakNetServer;
use rakrs::Motd;
use rakrs::RakResult;
use rakrs::RakNetEvent;
use rakrs::raknet_start;

fn main() {
     let mut server = RakNetServer::new(String::from("0.0.0.0:19132"));
     server.set_motd(Motd {
          name: "Netrex Raknet".to_owned(),
          protocol: 190,
          player_count: 0,
          player_max: 10000,
          gamemode: "creative".to_owned(),
          version: "1.18.9".to_owned(),
          server_id: 2747994720109207718 as i64
     });
     let threads = raknet_start!(server, move |ev: &RakNetEvent| {
          match ev.clone() {
               RakNetEvent::Disconnect(address, reason) => {
                    println!("{} disconnected due to: {}", address, reason);
                    None
               },
               RakNetEvent::ConnectionCreated(address) => {
                    println!("{} has joined the server", address);
                    None
               },
               RakNetEvent::GamePacket(address, _packet) => {
                    println!("{} sent a game packet!!", address);
                    Some(RakResult::Disconnect("Nope.".into()))
               },
               RakNetEvent::ComplexBinaryError(ip, _, msg) => {
                    println!("Err! {}: {}", ip, msg);
                    None
               }
               _ => None
          }
     });
     threads.0.join();
     threads.1.join();
     println!("Hi I am running concurrently.");
}