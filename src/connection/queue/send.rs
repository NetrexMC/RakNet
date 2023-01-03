use crate::protocol::ack::{Ack, Ackable, Record, SingleRecord};
use crate::protocol::frame::FramePacket;
use crate::util::SafeGenerator;

use super::{NetQueue, RecoveryQueue};

/// This queue is used to prioritize packets being sent out
/// Packets that are old, are either dropped or requested again.
/// You can define this behavior with the `timeout` property.
#[derive(Debug, Clone)]
pub struct SendQueue {
    /// The amount of time that needs to pass for a packet to be
    /// dropped or requested again.
    timeout: u16,

    /// The amount of times we should retry sending a packet before
    /// dropping it from the queue. This is currently set to `5`.
    max_tries: u16,

    /// The current sequence number. This is incremented every time
    /// a packet is sent reliably. We can resend these if they are
    /// NAcked.
    send_seq: u32,

    /// The current reliable index number.
    /// a packet is sent reliably an sequenced.
    reliable_seq: SafeGenerator<u16>,

    /// The current recovery queue.
    ack: RecoveryQueue<Vec<u8>>,
}

impl SendQueue {
    pub fn new() -> Self {
        Self {
            timeout: 5000,
            max_tries: 5,
            send_seq: 0,
            reliable_seq: SafeGenerator::new(),
            ack: RecoveryQueue::new(),
        }
    }

    pub fn insert(&mut self, packet: Vec<u8>, reliable: bool) -> u32 {
        // let next_id = self.fragment_seq.next();
        todo!()
    }

    pub fn next_seq(&mut self) -> u32 {
        let seq = self.send_seq;
        self.send_seq = self.send_seq.wrapping_add(1);
        return seq;
    }
}

impl Ackable for SendQueue {
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

    fn nack(&mut self, nack: Ack) -> Vec<Vec<u8>> {
        if !nack.is_nack() {
            return Vec::new();
        }

        let mut resend_queue = Vec::<Vec<u8>>::new();

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
