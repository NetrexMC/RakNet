use rakrs::RakNetServer;

fn main() {
     let mut server = RakNetServer::new(String::from("0.0.0.0:19132"));

     let threads = server.start();
     threads.0.join().unwrap();
     threads.1.join().unwrap();
     println!("Hi I am running");
}