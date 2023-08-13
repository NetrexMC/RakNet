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

struct DiscoveryState {
    status: DiscoveryStatus,
    waker: Option<Waker>,
}

pub struct MtuDiscovery {
    state: Arc<Mutex<DiscoveryState>>,
}

impl MtuDiscovery {}

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
