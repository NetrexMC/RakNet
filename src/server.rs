use crate::conn::{Connection, ConnectionState, RecievePacketFn};
use crate::util::{from_tokenized, tokenize_addr};
use crate::Motd;
use binary_utils::*;
use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

pub enum RakNetVersion {
     MinecraftRecent,
     V10,
     V6,
}

impl RakNetVersion {
     pub fn to_u8(&self) -> u8 {
          match self {
               RakNetVersion::MinecraftRecent => 10,
               RakNetVersion::V10 => 10,
               RakNetVersion::V6 => 6,
          }
     }
}

#[derive(Clone)]
pub enum RakNetEvent {
     /// When a connection is created
     ///
     /// ! This is not the same as connecting to the server !
     ///
     /// **Tuple Values**:
     /// 1. The parsed `ip:port` address of the connection.
     ConnectionCreated(String),
     /// When a connection disconnects from the server
     /// Or the server forces the connection to disconnect
     ///
     /// **Tuple Values**:
     /// 1. The parsed `ip:port` address of the connection.
     /// 2. The reason for disconnect.
     Disconnect(String, String),
     /// When a connection is sent a motd.
     ///
     /// **Tuple Values**:
     /// 1. The parsed `ip:port` address of the connection.
     MotdGeneration(String)
}

pub type RakEventListener = fn(event: &RakNetEvent);

pub struct RakNetServer {
     pub address: String,
     pub version: RakNetVersion,
     pub connections: Arc<Mutex<HashMap<String, Connection>>>,
     pub start_time: SystemTime,
     motd: Arc<Motd>,
     reciever: RecievePacketFn,
     listener: Option<RakEventListener>,
}

impl RakNetServer {
     pub fn new(address: String) -> Self {
          Self {
               address,
               version: RakNetVersion::MinecraftRecent,
               connections: Arc::new(Mutex::new(HashMap::new())),
               start_time: SystemTime::now(),
               motd: Arc::new(Motd::default()),
               reciever: |_: &mut Connection, _: &mut BinaryStream| {
                    println!("Default implmentation");
               },
               listener: None
          }
     }

     pub fn set_reciever(&mut self, recv: RecievePacketFn) {
          self.reciever = recv;
     }

     pub fn set_listener(&mut self, listener: RakEventListener) {
          self.listener = Some(listener);
     }

     pub fn set_motd(&mut self, motd: Motd) {
          *Arc::get_mut(&mut self.motd).unwrap() = motd;
     }

     /// Sends a stream to the specified address.
     /// Instant skips the tick and forcefully sends the packet to the client.
     pub fn send_stream(&mut self, address: String, stream: BinaryStream, instant: bool) {
          let clients = self.connections.lock();
          match clients.unwrap().get_mut(&address) {
               Some(c) => c.send(stream, instant),
               None => return,
          };
     }

     /// Starts a raknet server instance.
     /// Returns two thread handles, for both the send and recieving threads.
     pub fn start(&mut self) -> (thread::JoinHandle<()>, thread::JoinHandle<()>) {
          let socket = UdpSocket::bind(self.address.clone());
          let server_socket: Arc<UdpSocket> = Arc::new(socket.unwrap());
          let server_socket_1: Arc<UdpSocket> = Arc::clone(&server_socket);
          let clients_recv = Arc::clone(&self.connections);
          let clients_send = Arc::clone(&self.connections);
          let server_time = Arc::new(self.start_time);
          let caller = Arc::new(self.reciever);
          let motd = Arc::clone(&self.motd);
          let event_dispatch = Arc::new(self.listener);

          let recv_thread = thread::spawn(move || {
               let mut buf = [0; 2048];

               loop {
                    let (len, remote) = match server_socket.as_ref().recv_from(&mut buf) {
                         Ok(v) => v,
                         Err(_e) => continue,
                    };

                    let data = &buf[..len];
                    let mut stream = BinaryStream::init(&data.to_vec());
                    let mut sclients = clients_recv.lock().unwrap();

                    // check if a connection exists
                    if !sclients.contains_key(&tokenize_addr(remote)) {
                         // connection doesn't exist, make it
                         sclients.insert(
                              tokenize_addr(remote),
                              Connection::new(remote, *server_time.as_ref(), Arc::clone(&caller), Arc::clone(&motd)),
                         );
                    }

                    let client = match sclients.get_mut(&tokenize_addr(remote)) {
                         Some(c) => c,
                         None => continue,
                    };

                    client.recv(&mut stream);
               }
          });

          let sender_thread = thread::spawn(move || {
               loop {
                    thread::sleep(Duration::from_millis(50));
                    let clients = clients_send.lock().unwrap();
                    for (addr, client) in clients.clone().iter_mut() {
                         // emit events if there is a listener for them
                         if event_dispatch.is_some() {
                              let dispatch = event_dispatch.unwrap();
                              for event in client.event_dispatch.iter() {
                                   dispatch(event);
                              }
                         }

                         client.event_dispatch.clear();

                         if client.state == ConnectionState::Offline {
                              continue;
                         }

                         client.do_tick();

                         if client.send_queue.len() == 0 {
                              continue;
                         }

                         for pk in client.clone().send_queue.into_iter() {
                              match server_socket_1.as_ref().send_to(
                                   pk.get_buffer().as_slice(),
                                   &from_tokenized(addr.clone()),
                              ) {
                                   // Add proper handling!
                                   Err(_) => continue, //println!("Error Sending Packet [{}]: ", e),
                                   Ok(_) => continue, //println!("\nSent Packet [{}]: {:?}", addr, pk)
                              }
                         }
                         client.send_queue.clear();
                         drop(client);
                    }
                    drop(clients);
               }
          });
          return (sender_thread, recv_thread);
     }
}
