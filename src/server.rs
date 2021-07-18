use std::collections::HashMap;
use std::thread;
use std::sync::{ Arc, Mutex };
use std::time::SystemTime;
use std::net::UdpSocket;
use binary_utils::*;
use crate::conn::Connection;
use crate::util::tokenize_addr;
pub enum RakNetVersion {
     MinecraftRecent,
     V10,
     V6
}

impl RakNetVersion {
     pub fn to_u8(&self) -> u8 {
          match self {
               RakNetVersion::MinecraftRecent => 10,
               RakNetVersion::V10 => 10,
               RakNetVersion::V6 => 6
          }
     }
}

pub struct RakNetServer {
     pub address: String,
     pub version: RakNetVersion,
     pub connections: Arc<Mutex<HashMap<String, Connection>>>,
     pub start_time: SystemTime
}

impl RakNetServer {
     pub fn new(address: String) -> Self {
          Self {
               address,
               version: RakNetVersion::MinecraftRecent,
               connections: Arc::new(Mutex::new(HashMap::new())),
               start_time: SystemTime::now()
          }
     }

     pub fn start(&mut self) -> (thread::JoinHandle<()>, thread::JoinHandle<()>) {
          let socket = UdpSocket::bind(self.address.clone());
          let server_socket: Arc<UdpSocket> = Arc::new(socket.unwrap());
          let server_socket_1: Arc<UdpSocket> = Arc::clone(&server_socket);
          let clients = Arc::clone(&self.connections);
          let clients_mut = Arc::clone(&self.connections);
          let server_time = Arc::new(self.start_time);

          let recv_thread = thread::spawn(move || {
               let mut buf = [0; 1024];

               loop {
                    let (len, remote) = match server_socket.as_ref().recv_from(&mut buf) {
                         Ok(v) => v,
                         Err(_e) => continue
                    };

                    let data = &buf[..len];
                    let mut stream = BinaryStream::init(&data.to_vec());
                    let mut sclients = clients.lock().unwrap();

                    println!("\nGot Packet [{}]: {:?}", remote.to_string(), stream);

                    // check if a connection exists
                    if !sclients.contains_key(&tokenize_addr(remote)) {
                         // connection doesn't exist, make it
                         sclients.insert(tokenize_addr(remote), Connection::new(remote, *server_time.as_ref()));
                    }

                    let client = match sclients.get_mut(&tokenize_addr(remote)) {
                         Some(c) => c,
                         None => {
                              continue
                         }
                    };
                    client.receive(&mut stream);
               }
          });

          let sender_thread = thread::spawn(move || {
               loop {
                    let mut clients_t = clients_mut.lock().unwrap();
                    for (addr, _connect) in clients_t.clone().into_iter() {
                         let c = clients_t.get_mut(&addr).unwrap();
                         if c.send_queue.len() == 0 {
                              continue;
                         }

                         for pk in c.send_queue.clone().into_iter() {
                              match server_socket_1.as_ref().send_to(pk.get_buffer().as_slice(), &c.address) {
                                   // Add proper handling!
                                   Err(e) => println!("Error Sending Packet [{}]: ", e),
                                   Ok(_) => println!("\nSent Packet [{}]: {:?}", c.address, pk)
                              }
                         }
                         c.send_queue.clear();
                         drop(c);
                    }
               }
          });

          return (sender_thread, recv_thread);
     }
}