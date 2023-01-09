use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

#[cfg(feature = "async-std")]
use async_std::net::UdpSocket;
use binary_utils::Streamable;
#[cfg(feature = "tokio")]
use tokio::net::UdpSocket;

use crate::protocol::ack::{Ack, Ackable, Record, SingleRecord};
use crate::protocol::frame::{Frame, FramePacket};
use crate::protocol::packet::Packet;
use crate::protocol::reliability::Reliability;
use crate::protocol::{RAKNET_HEADER_FRAME_OVERHEAD, RAKNET_HEADER_OVERHEAD};
use crate::util::SafeGenerator;

use super::{FragmentQueue, NetQueue, OrderedQueue, RecoveryQueue};

/// This queue is used to prioritize packets being sent out
/// Packets that are old, are either dropped or requested again.
/// You can define this behavior with the `timeout` property.
#[derive(Debug, Clone)]
pub struct SendQueue {
    mtu_size: u16,

    /// The amount of time that needs to pass for a packet to be
    /// dropped or requested again.
    timeout: u16,

    /// The amount of times we should retry sending a packet before
    /// dropping it from the queue. This is currently set to `5`.
    max_tries: u16,

    /// The current sequence number. This is incremented every time
    /// a packet is sent reliably. We can resend these if they are
    /// NAcked.
    send_seq: SafeGenerator<u32>,

    /// The current reliable index number.
    /// a packet is sent reliably an sequenced.
    reliable_seq: SafeGenerator<u32>,

    /// The current recovery queue.
    ack: RecoveryQueue<FramePacket>,

    /// The fragment queue.
    fragment_queue: FragmentQueue,

    order_channels: HashMap<u8, OrderedQueue<Vec<u8>>>,

    ready: Vec<Frame>,

    socket: Arc<UdpSocket>,

    address: SocketAddr,
}

impl SendQueue {
    pub fn new(
        mtu_size: u16,
        timeout: u16,
        max_tries: u16,
        socket: Arc<UdpSocket>,
        address: SocketAddr,
    ) -> Self {
        Self {
            mtu_size,
            timeout,
            max_tries,
            send_seq: SafeGenerator::new(),
            reliable_seq: SafeGenerator::new(),
            ack: RecoveryQueue::new(),
            fragment_queue: FragmentQueue::new(),
            order_channels: HashMap::new(),
            ready: Vec::new(),
            socket,
            address,
        }
    }

    /// Send a packet based on its reliability.
    /// Note, reliability will be set to `Reliability::ReliableOrd` if
    /// the buffer is larger than max MTU.
    pub async fn insert(
        &mut self,
        packet: Vec<u8>,
        reliability: Reliability,
        channel: u8,
        immediate: bool,
    ) {
        let reliable = if packet.len() > (self.mtu_size + RAKNET_HEADER_FRAME_OVERHEAD) as usize {
            Reliability::ReliableOrd
        } else {
            reliability
        };

        // match reliability {
        //     Reliability::Unreliable => {
        //         // we can just send this packet out immediately.
        //         let frame = Frame::new(Reliability::Unreliable, Some(packet));
        //         self.send_frame(frame).await;
        //     },
        //     Reliability::Reliable => {
        //         // we need to send this packet out reliably.
        //         let frame = Frame::new(Reliability::Reliable, Some(packet));
        //         self.send_frame(frame).await;
        //     },
        // }
    }

    async fn send_frame(&mut self, mut frame: Frame) {
        let mut pk = FramePacket::new();
        pk.sequence = self.send_seq.next();
        pk.reliability = frame.reliability;

        if pk.reliability.is_reliable() {
            frame.reliable_index = Some(self.reliable_seq.next());
        }

        if let Ok(buf) = pk.parse() {
            self.send_socket(&buf[..]).await;
        }
    }

    async fn send_socket(&mut self, packet: &[u8]) {
        self.socket.send_to(packet, &self.address).await.unwrap();
    }

    pub async fn send_packet(&mut self, packet: Packet, immediate: bool) {
        // parse the packet
    }
}

impl Ackable for SendQueue {
    type NackItem = FramePacket;

    fn ack(&mut self, ack: Ack) {
        if ack.is_nack() {
            return;
        }

        // these packets are acknowledged, so we can remove them from the queue.
        for record in ack.records.iter() {
            match record {
                Record::Single(SingleRecord { sequence }) => {
                    self.ack.remove(*sequence);
                }
                Record::Range(ranged) => {
                    for i in ranged.start..ranged.end {
                        self.ack.remove(i);
                    }
                }
            }
        }
    }

    fn nack(&mut self, nack: Ack) -> Vec<FramePacket> {
        if !nack.is_nack() {
            return Vec::new();
        }

        let mut resend_queue = Vec::<FramePacket>::new();

        // we need to get the packets to resend.
        for record in nack.records.iter() {
            match record {
                Record::Single(single) => {
                    if let Ok(packet) = self.ack.get(single.sequence) {
                        resend_queue.push(packet.clone());
                    }
                }
                Record::Range(ranged) => {
                    for i in ranged.start..ranged.end {
                        if let Ok(packet) = self.ack.get(i) {
                            resend_queue.push(packet.clone());
                        }
                    }
                }
            }
        }

        return resend_queue;
    }
}
