use rakrs::RakNetServer;
use rakrs::conn::{Connection};
use binary_utils::*;

fn main() {
     let mut server = RakNetServer::new(String::from("0.0.0.0:19132"));
     server.set_reciever(|_con: &mut Connection, pk: &mut BinaryStream| {
          println!("Got game packet.");
     });
     let threads = server.start();
     threads.0.join();
     threads.1.join();
     println!("Hi I am running concurrently.");
}