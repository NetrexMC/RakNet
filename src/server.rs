use std::{ thread, io, net::{ SocketAddr, IpAddr }, sync::{ Mutex, Arc, mpsc::* } };
// use tokio::net::UdpSocket;
use std::net::UdpSocket;
use rand::random;
use binary_utils::*;
use super::conn::Connection;

pub const THREAD_QUIT: i64 = 982939013934;

pub trait IRakServer {
     fn new(address: String, version: u8) -> Self;

     fn start(&mut self);
}

pub struct RakServer {
     address: String,
     version: u8,
     connections: Vec<Connection>,
     send: Option<Sender<(SocketAddr, BinaryStream)>>,
     recv: Option<Receiver<(SocketAddr, BinaryStream)>>
}

impl IRakServer for RakServer {
     fn new(address: String, version: u8) -> Self {
          Self {
               version,
               address,
               connections: Vec::new(),
               send: None,
               recv: None
          }
     }

     fn start(&mut self) {
          let socket = UdpSocket::bind(self.address.parse::<SocketAddr>().unwrap());
          let resource: Arc<UdpSocket> = Arc::new(socket.unwrap());
          let (chan_send_s, chan_send_r) = channel::<(SocketAddr, BinaryStream)>();
          let (chan_recv_s, chan_recv_r) = channel::<(SocketAddr, BinaryStream)>();

          self.send = Some(chan_send_s);
          self.recv = Some(chan_recv_r);

          // sending thread
          let threads = {
               let res = Arc::clone(&resource);
               let send_thread = thread::spawn(move || {
                    loop {
                         let (address, stream) = match chan_send_r.try_recv() {
                              Ok(t) => t,
                              Err(TryRecvError) => panic!("Could not send to client.")
                         };
                         let mut st = BinaryStream::init(&stream.get_buffer());

                         if address.ip().is_loopback() && st.read_byte() == 0xCE && st.read_long() == THREAD_QUIT {
                              println!("Recieved quit message");
                              break;
                         }

                         // let tr = resource.lock().unwrap();
                         res.as_ref().send_to(&*stream.get_buffer(), address).expect("Could not send bytes to client.");
                    }
               });

               let recieve_thread = thread::spawn(move || {
                    let mut buf = [0; 65535];
                    loop {
                         // let tr = resource.lock().unwrap();
                         let (len, rem) = match resource.as_ref().recv_from(&mut buf) {
                              Ok(v) => v,
                              Err(e) => {
                                   continue;
                              }
                         };

                         let data = &buf[..len];
                         chan_recv_s.send((rem, BinaryStream::init(&data.to_vec()))).unwrap();
                    }
               });
          };
     }
}