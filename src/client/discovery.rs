use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

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
#[cfg(feature = "async_tokio")]
use std::task::{Context, Poll, Waker};
#[cfg(feature = "async_tokio")]
use tokio::{
    net::UdpSocket,
    task::{self},
    time::timeout,
};

use crate::match_ids;
use crate::protocol::packet::offline::IncompatibleProtocolVersion;
use crate::protocol::packet::offline::OpenConnectReply;
use crate::protocol::packet::offline::OpenConnectRequest;
use crate::rakrs_debug;

use super::util::send_packet;

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
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
    }};
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiscoveryStatus {
    /// The discovery has been initiated.
    /// This only occurs when the discovery is first created.
    Initiated,
    /// The discovery has been completed.
    /// We know the MTU size.
    Discovered(u16),
    /// We failed to discover the MTU size.
    /// This is probably cause the server is offline.
    Failed,
    /// The discovery has been canceled because the client is on a different protocol version.
    IncompatibleVersion,
    /// We're still trying to find the MTU size.
    Undiscovered,
}

impl std::fmt::Display for DiscoveryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                DiscoveryStatus::Initiated => "Initiated",
                DiscoveryStatus::Discovered(_) => "Discovered",
                DiscoveryStatus::Failed => "Failed",
                DiscoveryStatus::IncompatibleVersion => "IncompatibleVersion",
                DiscoveryStatus::Undiscovered => "Undiscovered",
            }
        )
    }
}

#[derive(Debug, Clone)]
pub struct MtuDiscoveryMeta {
    pub id: i64,
    pub version: u8,
    pub mtu: u16,
    pub timeout: u16,
}

struct DiscoveryState {
    status: DiscoveryStatus,
    waker: Option<Waker>,
}

pub struct MtuDiscovery {
    state: Arc<Mutex<DiscoveryState>>,
}

impl MtuDiscovery {
    pub fn new(socket: Arc<UdpSocket>, discovery_info: MtuDiscoveryMeta) -> Self {
        let state = Arc::new(Mutex::new(DiscoveryState {
            status: DiscoveryStatus::Initiated,
            waker: None,
        }));

        let shared_state = state.clone();

        task::spawn(async move {
            // try to use the mtu provided by the user
            let valid_mtus: Vec<u16> = vec![discovery_info.mtu, 1506, 1492, 1400, 1200, 576];
            // remove any mtu size that is larger than discovery_info.mtu
            // let valid_mtus: Vec<u16> = valid_mtus
            //     .into_iter()
            //     .filter(|mtu| *mtu <= discovery_info.mtu)
            //     .collect();
            for mtu in valid_mtus.iter() {
                // send a connection request
                let request = OpenConnectRequest {
                    protocol: discovery_info.version,
                    mtu_size: *mtu,
                };

                if !send_packet(&socket, request.into()).await {
                    rakrs_debug!(
                        true,
                        "[CLIENT] Failed sending OpenConnectRequest to server!"
                    );
                    update_state!(shared_state, DiscoveryStatus::Undiscovered);
                    // this is ok! we'll just try the next mtu
                    continue;
                };

                let reply = match_ids!(
                    socket.clone(),
                    discovery_info.timeout.into(),
                    // Open connect Reply
                    0x06,
                    // Incompatible protocol version
                    0x19
                );

                if reply.is_none() {
                    update_state!(shared_state, DiscoveryStatus::Undiscovered);
                    // break;
                    continue;
                }

                // this is being triggered, why not returning???
                if let Ok(pk) = IncompatibleProtocolVersion::read(&mut ByteReader::from(
                    &reply.clone().unwrap()[1..],
                )) {
                    rakrs_debug!(
                        true,
                        "[CLIENT] Protocol mismatch. Server={}, Client={}",
                        pk.protocol,
                        discovery_info.version
                    );
                    update_state!(shared_state, DiscoveryStatus::IncompatibleVersion);
                    return;
                }

                let open_reply =
                    OpenConnectReply::read(&mut ByteReader::from(&reply.unwrap()[1..]));

                if open_reply.is_err() {
                    update_state!(shared_state, DiscoveryStatus::Failed);
                    return;
                }

                if let Ok(response) = open_reply {
                    rakrs_debug!(
                        true,
                        "[CLIENT] Received OpenConnectReply from server! mtu={}",
                        response.mtu_size
                    );
                    let mtu = response.mtu_size;
                    update_state!(shared_state, DiscoveryStatus::Discovered(mtu));
                    return;
                } else {
                    update_state!(shared_state, DiscoveryStatus::Undiscovered);
                }
            }

            update_state!(shared_state, DiscoveryStatus::Failed);
        });

        Self { state }
    }
}

impl Future for MtuDiscovery {
    type Output = DiscoveryStatus;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.lock().unwrap();
        match state.status {
            DiscoveryStatus::Failed
            | DiscoveryStatus::IncompatibleVersion
            | DiscoveryStatus::Discovered(_) => Poll::Ready(state.status),
            _ => {
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
