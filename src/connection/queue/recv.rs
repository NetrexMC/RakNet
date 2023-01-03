use binary_utils::*;
use std::collections::{HashMap, HashSet};

use crate::connection::controller::window::ReliableWindow;
use crate::protocol::ack::{Ack, Ackable, Record, SingleRecord};
use crate::protocol::frame::{Frame, FramePacket};
use crate::protocol::reliability::Reliability;
use crate::protocol::MAX_FRAGS;
use crate::rakrs_debug;
use crate::server::current_epoch;

use super::{FragmentQueue, OrderedQueue};

#[derive(Debug, Clone)]
pub enum RecvQueueError {
    OldSeq,
}

#[derive(Debug, Clone)]
pub struct RecvQueue {
    frag_queue: FragmentQueue,
    window: ReliableWindow,
    reliable_window: ReliableWindow,
    order_channels: HashMap<u8, OrderedQueue<Vec<u8>>>,
    /// Set of sequences that we've acknowledged.
    /// (seq, time)
    ack: HashSet<(u32, u64)>,
    ready: Vec<Vec<u8>>,
}

impl RecvQueue {
    pub fn new() -> Self {
        Self {
            frag_queue: FragmentQueue::new(),
            ack: HashSet::new(),
            window: ReliableWindow::new(),
            reliable_window: ReliableWindow::new(),
            ready: Vec::new(),
            order_channels: HashMap::new(),
        }
    }

    pub fn insert(&mut self, packet: FramePacket) -> Result<(), RecvQueueError> {
        if !self.window.insert(packet.sequence) {
            return Err(RecvQueueError::OldSeq);
        }

        self.ack.insert((packet.sequence, current_epoch()));

        for frame in packet.frames.iter() {
            self.handle_frame(frame);
        }

        return Ok(());
    }

    pub fn flush(&mut self) -> Vec<Vec<u8>> {
        self.ready.drain(..).collect::<Vec<Vec<u8>>>()
    }

    fn handle_frame(&mut self, frame: &Frame) {
        if let Some(reliable_index) = frame.reliable_index {
            if !self.reliable_window.insert(reliable_index) {
                return;
            }
        }
        if let Some(meta) = frame.fragment_meta.as_ref() {
            if meta.size > MAX_FRAGS {
                rakrs_debug!(true, "Fragment size is too large, rejected {}!", meta.size);
                return;
            }
            if let Err(_) = self.frag_queue.insert(frame.clone()) {}

            let res = self.frag_queue.collect(meta.id);
            if let Ok(data) = res {
                // reconstructed frame packet!
                self.ready.push(data);
            }
            return;
        }
        match frame.reliability {
            Reliability::Unreliable => {
                self.ready.push(frame.body.clone());
            }
            Reliability::Reliable => {
                self.ready.push(frame.body.clone());
            }
            Reliability::ReliableOrd => {
                let channel = frame.order_channel.unwrap();
                let queue = self
                    .order_channels
                    .entry(channel)
                    .or_insert(OrderedQueue::new());

                if !queue.insert(frame.order_index.unwrap(), frame.body.clone()) {
                    return;
                }

                for pk in queue.flush() {
                    self.ready.push(pk);
                }
            }
            _ => {
                self.ready.push(frame.body.clone());
            }
        }
    }
}
