use crate::protocol::packet::RakPacket;
use crate::{rakrs_debug, rakrs_debug_buffers};
#[cfg(feature = "async_std")]
use async_std::net::UdpSocket;
use binary_util::interfaces::Writer;
use std::sync::Arc;
#[cfg(feature = "async_tokio")]
use tokio::net::UdpSocket;

pub async fn send_packet(socket: &Arc<UdpSocket>, packet: RakPacket) -> bool {
    if let Ok(buf) = packet.write_to_bytes() {
        if let Err(e) = socket
            .send_to(buf.as_slice(), socket.peer_addr().unwrap())
            .await
        {
            rakrs_debug!("[CLIENT] Failed sending payload to server! {}", e);
            return false;
        } else {
            rakrs_debug_buffers!(false, "[annon]\n{:?}", buf.as_slice());
            return true;
        }
    } else {
        rakrs_debug!("[CLIENT] Failed writing payload to bytes!");
        return false;
    }
}
