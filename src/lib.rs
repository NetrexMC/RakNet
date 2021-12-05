#![feature(cursor_remaining)]
extern crate binary_utils;

pub mod ack;
pub mod conn;
pub mod frame;
pub mod protocol;
pub mod server;
pub mod util;

pub const MAGIC: [u8; 16] = [
    0x00, 0xff, 0xff, 0x0, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78,
];
pub const SERVER_ID: i64 = 2747994720109207718; //rand::random::<i64>();
pub const USE_SECURITY: bool = false;
pub type RakEventListenerFn = dyn FnMut(&RakNetEvent) -> Option<RakResult> + Send + Sync;

pub use self::{frame::*, protocol::*, server::*, util::*};

#[macro_export]
macro_rules! raknet_start {
    ($server: expr, $channel: expr) => {{
        use netrex_events::Channel;
        use rakrs::conn::{Connection, ConnectionState};
        use rakrs::util::{from_tokenized, tokenize_addr};
        use std::net::UdpSocket;
        use std::sync::Arc;
        use std::thread;
        use std::time::Duration;

        let socket = UdpSocket::bind($server.address.clone());
        let server_socket: Arc<UdpSocket> =
            Arc::new(socket.expect("Something is already using this socket address."));
        let server_socket_1: Arc<UdpSocket> = Arc::clone(&server_socket);
        let clients_recv = Arc::clone(&$server.connections);
        let clients_send = Arc::clone(&$server.connections);
        let server_time = Arc::new($server.start_time);
        let motd = Arc::clone(&$server.motd);
        let send_channel = Arc::new($channel);

        let handle = crossbeam::scope(|scope| {
            // server thread
            scope.spawn(|_| {
                let mut buf = [0; 2048];
                loop {
                    let (len, remote) = match server_socket.as_ref().recv_from(&mut buf) {
                        Ok(v) => v,
                        Err(_e) => continue,
                    };

                    let data = &buf[..len];
                    let mut sclients = clients_recv.lock().unwrap();

                    // check if a connection exists
                    if !sclients.contains_key(&tokenize_addr(remote)) {
                        // connection doesn't exist, make it
                        sclients.insert(
                            tokenize_addr(remote),
                            Connection::new(remote, *server_time.as_ref(), Arc::clone(&motd)),
                        );
                    }

                    let client = match sclients.get_mut(&tokenize_addr(remote)) {
                        Some(c) => c,
                        None => continue,
                    };

                    client.recv(&data.to_vec());
                }
            });

            scope.spawn(|_| {
                loop {
                    thread::sleep(Duration::from_millis(50));
                    let mut clients = clients_send.lock().unwrap();
                    for (addr, _) in clients.clone().iter() {
                        let client = clients.get_mut(addr).expect("Could not get connection");
                        client.do_tick();

                        let dispatch = client.event_dispatch.clone();
                        client.event_dispatch.clear();

                        // emit events if there is a listener for the
                        for event in dispatch.iter() {
                            // println!("DEBUG => Dispatching: {:?}", &event.get_name());
                            if let Some(result) = send_channel.send(event.clone()) {
                                match result {
                                    RakResult::Motd(_v) => {
                                        // we don't really support changing
                                        // client MOTD at the moment...
                                        // so we don't do anything for this.
                                    }
                                    RakResult::Error(v) => {
                                        // Calling error forces an error to raise.
                                        panic!("{}", v);
                                    }
                                    RakResult::Disconnect(_) => {
                                        client.state = ConnectionState::Offline; // simple hack
                                        break;
                                    }
                                }
                            }
                        }

                        if client.state == ConnectionState::Offline {
                            clients.remove(addr);
                            continue;
                        }

                        if client.send_queue.len() == 0 {
                            continue;
                        }

                        for pk in client.clone().send_queue.into_iter() {
                            match server_socket_1
                                .as_ref()
                                .send_to(&pk[..], &from_tokenized(addr.clone()))
                            {
                                // Add proper handling!
                                Err(e) => eprintln!("Error Sending Packet [{}]: ", e),
                                Ok(_) => continue, // println!("\nSent Packet [{}]: {:?}", addr, pk)
                            }
                        }
                        client.send_queue.clear();
                    }
                    drop(clients);
                }
            });
        })
        .unwrap();
    }};
}

#[cfg(test)]
mod tests {
    use crate::RakNetServer;

    #[test]
    fn rak_serv() {
        let mut _serve = RakNetServer::new(String::from("0.0.0.0:19132"));
    }
}
