use std::collections::HashMap;
use std::thread;
use std::sync::{ Arc, Mutex, MutexGuard };
use std::cell::RefCell;
use std::net::UdpSocket;
use binary_utils::*;
use crate::conn::Connection;
use crate::conn::ConnectionAPI;
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
     pub id: u64,
     pub address: String,
     pub version: RakNetVersion,
     pub connections: Arc<Mutex<HashMap<String, Connection>>>
}

impl RakNetServer {
     pub fn new(address: String) -> Self {
          Self {
               id: rand::random::<u64>(),
               address,
               version: RakNetVersion::MinecraftRecent,
               connections: Arc::new(Mutex::new(HashMap::new()))
          }
     }

     pub fn start(&mut self) -> (thread::JoinHandle<()>, thread::JoinHandle<()>) {
          let socket = UdpSocket::bind(self.address.clone());
          let server_socket: Arc<UdpSocket> = Arc::new(socket.unwrap());
          let server_socket_1: Arc<UdpSocket> = Arc::clone(&server_socket);
          let clients = Arc::clone(&self.connections);
          let clients_mut = Arc::clone(&self.connections);

          let recv_thread = thread::spawn(move || {
               let mut buf = [0; 1024];

               loop {
                    let (len, remote) = match server_socket.as_ref().recv_from(&mut buf) {
                         Ok(v) => v,
                         Err(_e) => continue
                    };

                    let data = &buf[..len];
                    let stream = BinaryStream::init(&data.to_vec());
                    let mut sclients = clients.lock().unwrap();

                    println!("Got Packet [{}]: {:?}", remote.to_string(), stream);

                    let exists = match sclients.get(&remote.to_string()) {
                         Some(v) => true,
                         None => false
                    };

                    // check if a connection exists
                    if !exists {
                         // connection doesn't exist, make it
                         sclients.insert(tokenize_addr(remote), Connection::new(remote));
                    }

                    let client = match sclients.get_mut(&remote.to_string()) {
                         Some(c) => c,
                         None => continue
                    };
                    client.receive(&stream);
               }
          });

          let sender_thread = thread::spawn(move || {
               loop {
                    let mut clients = clients_mut.lock().unwrap();
                    for (addr, _connect) in clients.clone().into_iter() {
                         let c = clients.get_mut(&addr).unwrap();
                         if c.send_queue.len() == 0 {
                              continue;
                         }
                         for pk in c.send_queue.clone().into_iter() {
                              match server_socket_1.send_to(pk.get_buffer().as_slice(), &addr) {
                                   // Add proper handling!
                                   Err(_) => continue,
                                   Ok(_) => continue
                              }
                         }
                         c.send_queue.clear();
                    }
               }
          });

          return (sender_thread, recv_thread);
     }
}