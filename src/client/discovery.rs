#[cfg(feature = "async_std")]
use async_std::{
    future::timeout,
    future::Future,
    net::UdpSocket,
    task::{self, Context, Poll, Waker},
};

#[cfg(feature = "async_tokio")]
use std::future::Future;
use std::sync::{Arc, Mutex};
#[cfg(feature = "async_tokio")]
use std::task::{Context, Poll, Waker};
#[cfg(feature = "async_tokio")]
use tokio::{
    net::UdpSocket,
    task::{self},
    time::timeout,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiscoveryStatus {
    Initiated,
    Discovered,
    Failed,
    Undiscovered,
}

#[derive(Debug, Clone)]
pub struct MtuDiscoveryMeta {
    pub id: i64,
    pub version: u8,
    pub mtu: u16
}

struct DiscoveryState {
    status: DiscoveryStatus,
    waker: Option<Waker>,
}

pub struct MtuDiscovery {
    state: Arc<Mutex<DiscoveryState>>,
}

impl MtuDiscovery {
    pub fn new(socket: Arc<UdpSocket>, mut discovery_info: MtuDiscoveryMeta) -> Self {
        let state = Arc::new(Mutex::new(DiscoveryState {
            status: DiscoveryStatus::Initiated,
            waker: None,
        }));

        let shared_state = state.clone();

        task::spawn(async move {
            let mut buf = [0u8; 1024];
            
            // try to use the mtu provided by the user
            loop {
                
            }
        });

        Self { state }
    }
}

impl Future for MtuDiscovery {
    type Output = DiscoveryStatus;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.lock().unwrap();
        match state.status {
            DiscoveryStatus::Failed | DiscoveryStatus::Discovered => Poll::Ready(state.status),
            _ => {
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
