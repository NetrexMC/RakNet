use crate::client::discovery;
use crate::client::discovery::DiscoveryStatus;
use crate::client::discovery::MtuDiscovery;
use crate::client::util::send_packet;
use crate::connection::queue::send::SendQueue;
use crate::connection::queue::RecvQueue;
use crate::protocol::frame::FramePacket;
use crate::protocol::packet::offline::{SessionInfoReply, SessionInfoRequest};
use crate::protocol::packet::online::ConnectedPong;
use crate::protocol::packet::online::{ConnectionRequest, NewConnection, OnlinePacket};
use crate::protocol::reliability::Reliability;
use crate::protocol::Magic;
use crate::rakrs_debug;
use crate::server::current_epoch;
#[cfg(feature = "async_std")]
use async_std::{
    future::timeout,
    future::Future,
    net::UdpSocket,
    task::{self, Context, Poll, Waker},
};
use binary_util::interfaces::Reader;
use binary_util::io::ByteReader;
#[cfg(feature = "async_tokio")]
use std::future::Future;
use std::sync::Arc;
use std::sync::Mutex;
#[cfg(feature = "async_tokio")]
use std::task::{Context, Poll, Waker};
use std::time::Duration;
#[cfg(feature = "async_tokio")]
use tokio::{
    net::UdpSocket,
    task::{self},
    time::timeout,
};

#[macro_export]
macro_rules! match_ids {
    ($socket: expr, $timeout: expr, $($ids: expr),*) => {
        {
            let mut recv_buf: [u8; 2048] = [0; 2048];
            let mut tries: u8 = 0;
            let ids = vec![$($ids),*];
            let mut pk: Option<Vec<u8>> = None;

            'try_conn: loop {
                if (tries >= 5) {
                    break;
                }

                let len: usize;
                let send_result = timeout(
                    Duration::from_secs($timeout),
                    $socket.recv(&mut recv_buf)
                ).await;

                if (send_result.is_err()) {
                    rakrs_debug!(true, "[CLIENT] Failed to receive packet from server! Is it offline?");
                    break 'try_conn;
                }

                match send_result.unwrap() {
                    Err(e) => {
                        tries += 1;
                        rakrs_debug!(true, "[CLIENT] Failed to receive packet from server! {}", e);
                        continue;
                    },
                    Ok(l) => len = l
                };

                crate::rakrs_debug_buffers!(true, "[annon]\n {:?}", &recv_buf[..len]);

                // rakrs_debug!(true, "[CLIENT] Received packet from server: {:x?}", &recv_buf[..len]);

                if ids.contains(&recv_buf[0]) {
                    pk = Some(recv_buf[..len].to_vec());
                    break 'try_conn;
                }
            }

            pk
        }
    };
}

macro_rules! expect_reply {
    ($socket: expr, $reply: ty, $timeout: expr) => {{
        let mut recv_buf: [u8; 2048] = [0; 2048];
        let mut tries: u8 = 0;
        let mut pk: Option<$reply> = None;

        loop {
            if (tries >= 5) {
                break;
            }

            let len: usize;
            let send_result =
                timeout(Duration::from_secs($timeout), $socket.recv(&mut recv_buf)).await;

            if (send_result.is_err()) {
                rakrs_debug!(
                    true,
                    "[CLIENT] Failed to receive packet from server! Is it offline?"
                );
                break;
            }

            match send_result.unwrap() {
                Err(_) => {
                    tries += 1;
                    continue;
                }
                Ok(l) => len = l,
            };

            // rakrs_debug!(true, "[CLIENT] Received packet from server: {:x?}", &recv_buf[..len]);
            crate::rakrs_debug_buffers!(true, "[annon]\n {:?}", &recv_buf[..len]);

            let mut reader = ByteReader::from(&recv_buf[1..len]);
            if let Ok(packet) = <$reply>::read(&mut reader) {
                pk = Some(packet);
                break;
            } else {
                rakrs_debug!(true, "[CLIENT] Failed to parse packet!");
            }
        }

        pk
    }};
}

macro_rules! update_state {
    ($done: expr, $shared_state: expr, $state: expr) => {{
        let mut state = $shared_state.lock().unwrap();
        state.status = $state;
        state.done = true;
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
        return;
    }};
    ($shared_state: expr, $state: expr) => {{
        let mut state = $shared_state.lock().unwrap();
        state.status = $state;
        state.done = false;
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
    }};
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HandshakeStatus {
    Created,
    Opening,
    SessionOpen,
    Failed,
    FailedMtuDiscovery,
    FailedNoSessionReply,
    IncompatibleVersion,
    Completed,
}

impl std::fmt::Display for HandshakeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                HandshakeStatus::Created => "Handshake created",
                HandshakeStatus::Opening => "Opening handshake",
                HandshakeStatus::SessionOpen => "Session open",
                HandshakeStatus::Failed => "Handshake failed",
                HandshakeStatus::FailedMtuDiscovery => "MTU discovery failed",
                HandshakeStatus::FailedNoSessionReply => "No session reply",
                HandshakeStatus::IncompatibleVersion => "Incompatible version",
                HandshakeStatus::Completed => "Handshake completed",
            }
        )
    }
}

pub(crate) struct HandshakeState {
    status: HandshakeStatus,
    done: bool,
    waker: Option<Waker>,
}

pub struct ClientHandshake {
    status: Arc<Mutex<HandshakeState>>,
}

impl ClientHandshake {
    pub fn new(
        socket: Arc<UdpSocket>,
        id: i64,
        version: u8,
        mut mtu: u16,
        attempts: u8,
        timeout: u16,
    ) -> Self {
        let state = Arc::new(Mutex::new(HandshakeState {
            done: false,
            status: HandshakeStatus::Created,
            waker: None,
        }));

        let shared_state = state.clone();

        task::spawn(async move {
            update_state!(shared_state, HandshakeStatus::Opening);

            rakrs_debug!(true, "[CLIENT] Sending OpenConnectRequest to server...");

            match MtuDiscovery::new(
                socket.clone(),
                discovery::MtuDiscoveryMeta {
                    id,
                    version,
                    mtu,
                    timeout,
                },
            )
            .await
            {
                DiscoveryStatus::Discovered(m) => {
                    rakrs_debug!(true, "[CLIENT] Discovered MTU size: {}", m);
                    mtu = m;
                }
                DiscoveryStatus::IncompatibleVersion => {
                    rakrs_debug!(
                        true,
                        "[CLIENT] Client is using incompatible protocol version."
                    );
                    update_state!(true, shared_state, HandshakeStatus::IncompatibleVersion);
                }
                _ => {
                    update_state!(true, shared_state, HandshakeStatus::FailedMtuDiscovery);
                }
            }

            let session_info = SessionInfoRequest {
                magic: Magic::new(),
                address: socket.peer_addr().unwrap(),
                mtu_size: mtu,
                client_id: id,
            };

            rakrs_debug!(true, "[CLIENT] Sending SessionInfoRequest to server...");

            update_state!(shared_state, HandshakeStatus::SessionOpen);

            if !send_packet(&socket, session_info.into()).await {
                rakrs_debug!(
                    true,
                    "[CLIENT] Failed to send SessionInfoRequest to server."
                );
                update_state!(true, shared_state, HandshakeStatus::Failed);
            }

            let session_reply = expect_reply!(socket, SessionInfoReply, timeout.into());

            if session_reply.is_none() {
                rakrs_debug!(true, "[CLIENT] Server did not reply with SessionInfoReply!");
                update_state!(true, shared_state, HandshakeStatus::FailedNoSessionReply);
            }

            let session_reply = session_reply.unwrap();

            if session_reply.mtu_size != mtu {
                rakrs_debug!(
                    true,
                    "[CLIENT] Server replied with incompatible MTU size! ({} != {})",
                    session_reply.mtu_size,
                    mtu
                );
                update_state!(true, shared_state, HandshakeStatus::Failed);
            }

            rakrs_debug!(true, "[CLIENT] Received SessionInfoReply from server!");

            // create a temporary sendq
            let mut send_q = SendQueue::new(
                mtu,
                timeout,
                attempts.clone().into(),
                socket.clone(),
                socket.peer_addr().unwrap(),
            );
            let mut recv_q = RecvQueue::new();

            if let Err(_) = Self::send_connection_request(&mut send_q, id).await {
                update_state!(true, shared_state, HandshakeStatus::Failed);
            }

            rakrs_debug!(true, "[CLIENT] Sent ConnectionRequest to server!");

            let mut send_time = current_epoch() as i64;
            let mut tries = 0_u8;

            let mut buf: [u8; 2048] = [0; 2048];

            loop {
                let len: usize;
                let rec = socket.recv_from(&mut buf).await;

                if (send_time + 2) <= current_epoch() as i64 {
                    send_time = current_epoch() as i64;

                    rakrs_debug!(
                        true,
                        "[CLIENT] Server did not reply with ConnectAccept, sending another..."
                    );

                    if let Err(_) = Self::send_connection_request(&mut send_q, id).await {
                        update_state!(true, shared_state, HandshakeStatus::Failed);
                    }

                    tries += 1;
                    if tries >= 5 {
                        update_state!(true, shared_state, HandshakeStatus::Failed);
                    }
                }

                match rec {
                    Err(_) => {
                        continue;
                    }
                    Ok((l, _)) => len = l,
                };

                let mut reader = ByteReader::from(&buf[..len]);

                // proccess frame packet
                match buf[0] {
                    0x80..=0x8d => {
                        if let Ok(pk) = FramePacket::read(&mut reader) {
                            if let Err(_) = recv_q.insert(pk) {
                                continue;
                            }

                            let raw_packets = recv_q.flush();

                            for raw_pk in raw_packets {
                                let mut pk = ByteReader::from(&raw_pk[..]);

                                if let Ok(pk) = OnlinePacket::read(&mut pk) {
                                    match pk {
                                        OnlinePacket::ConnectedPing(pk) => {
                                            rakrs_debug!(
                                                true,
                                                "[CLIENT] Received ConnectedPing from server!"
                                            );
                                            let response = ConnectedPong {
                                                ping_time: pk.time,
                                                pong_time: current_epoch() as i64,
                                            };

                                            if let Err(_) = send_q
                                                .send_packet(
                                                    response.into(),
                                                    Reliability::Reliable,
                                                    true,
                                                )
                                                .await
                                            {
                                                rakrs_debug!(
                                                    true,
                                                    "[CLIENT] Failed to send pong packet!"
                                                );
                                            }

                                            continue;
                                        }
                                        OnlinePacket::ConnectionAccept(pk) => {
                                            // send new incoming connection
                                            let new_incoming = NewConnection {
                                                server_address: socket.peer_addr().unwrap(),
                                                system_address: vec![
                                                    socket.peer_addr().unwrap(),
                                                    socket.peer_addr().unwrap(),
                                                    socket.peer_addr().unwrap(),
                                                    socket.peer_addr().unwrap(),
                                                    socket.peer_addr().unwrap(),
                                                    socket.peer_addr().unwrap(),
                                                    socket.peer_addr().unwrap(),
                                                    socket.peer_addr().unwrap(),
                                                    socket.peer_addr().unwrap(),
                                                    socket.peer_addr().unwrap(),
                                                ],
                                                request_time: pk.request_time,
                                                timestamp: pk.timestamp,
                                            };
                                            if let Err(_) = send_q
                                                .send_packet(
                                                    new_incoming.into(),
                                                    Reliability::Reliable,
                                                    true,
                                                )
                                                .await
                                            {
                                                update_state!(
                                                    true,
                                                    shared_state,
                                                    HandshakeStatus::Failed
                                                );
                                            } else {
                                                update_state!(
                                                    true,
                                                    shared_state,
                                                    HandshakeStatus::Completed
                                                );
                                            }
                                        }
                                        _ => {
                                            rakrs_debug!(
                                                true,
                                                "[CLIENT] Received unknown packet from server!"
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        Self { status: state }
    }

    pub(crate) async fn send_connection_request(
        send_q: &mut SendQueue,
        id: i64,
    ) -> std::io::Result<()> {
        let connect_request = ConnectionRequest {
            time: current_epoch() as i64,
            client_id: id,
            security: false,
        };

        if let Err(_) = send_q
            .send_packet(connect_request.into(), Reliability::Reliable, true)
            .await
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to send ConnectionRequest!",
            ));
        }
        return Ok(());
    }
}

impl Future for ClientHandshake {
    type Output = HandshakeStatus;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // see if we can finish
        let mut state = self.status.lock().unwrap();

        if state.done {
            return Poll::Ready(state.status);
        } else {
            state.waker = Some(cx.waker().clone());
            return Poll::Pending;
        }
    }
}
