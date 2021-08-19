use rakrs::RakNetServer;
use rakrs::conn::{Connection};
use rakrs::Motd;
use rakrs::RakNetEvent;
use binary_utils::*;
use std::sync::{Arc};

fn main() {
     let mut server = RakNetServer::new(String::from("0.0.0.0:19132"));
     server.set_motd(Motd {
          name: "Sus!!!".to_owned(),
          protocol: 190,
          player_count: 0,
          player_max: 10000,
          gamemode: "creative".to_owned(),
          version: "1.18.9".to_owned(),
          server_id: 2747994720109207718 as i64
     });
     let threads = server.start(Arc::new(|_con: &mut Connection, pk: &mut BinaryStream| {
               println!("Got game packet.");
          }), Box::new(|ev: &RakNetEvent| {
          match ev.clone() {
               RakNetEvent::Disconnect(address, reason) => {
                    println!("{} disconnected due to: {}", address, reason);
               },
               RakNetEvent::ConnectionCreated(address) => {
                    println!("{} has joined the server", address);
               }
               _ => return
          }
     }));
     threads.0.join();
     threads.1.join();
     println!("Hi I am running concurrently.");
}