use rakrs::RakNetServer;
use rakrs::conn::{Connection};
use binary_utils::*;

fn main() {
     let mut server = RakNetServer::new(String::from("0.0.0.0:19132"));
     server.set_reciever(|con: &mut Connection, pk: &mut BinaryStream| {
          println!("Got byte: {}", pk.read_byte());
     });
     let threads = server.start();
     threads.0.join().unwrap();
     threads.1.join().unwrap();
     println!("Hi I am running");
}