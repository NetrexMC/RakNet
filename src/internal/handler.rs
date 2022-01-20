use binary_utils::*;
use std::collections::{HashMap, HashSet};

use crate::connection::Connection;

use super::{
    ack::{Ack, Record},
    frame::{Frame, FramePacket},
    queue::BufferQueue,
};

pub enum RakHandlerError {
    Unknown(String),
    BinaryError(binary_utils::error::BinaryError),
}

impl From<String> for RakHandlerError {
    fn from(s: String) -> Self {
        RakHandlerError::Unknown(s)
    }
}

impl From<binary_utils::error::BinaryError> for RakHandlerError {
    fn from(e: binary_utils::error::BinaryError) -> Self {
        RakHandlerError::BinaryError(e)
    }
}

/// The handler for Ack, Nack and Frame packets.
/// This does not handle the actual sending of packets,
pub struct RakSessionHandler {
    /// The next Non-Acked packets that should be sent.
    /// These are packets we expect back from the client, but have not gotten.
    pub nack: HashSet<u32>,
    /// The Acked packets that have been sent, waiting for ack back. (only if reliable)
    /// This is used to determine if the packet has been received.
    /// We're also storing the count of how many times we've sent a packet.
    /// If it's been sent more than once, we'll consider it lost and remove it.
    pub ack: HashMap<u32, Vec<u8>>,
    /// A queue to send back to the client to acknowledge we've recieved these packets.
    pub ack_counts: HashSet<u32>,
    /// The ordered channels that have been recieved and are waiting for completion.
    /// Ordered channels will be reorded once all the packets have been received.
    pub ordered_channels: HashMap<u8, HashMap<u16, Frame>>,
    /// The fragmented frames that are waiting for reassembly.
    pub fragmented_frames: HashMap<u32, HashMap<u32, Frame>>,
    /// The last recieved sequence id.
    pub recv_win_start: u32,
    /// The end of the recieving window
    pub recv_win_end: u32,
    /// The highest most recently received sequence number
    pub recv_win_high: u32,
}

impl RakSessionHandler {
    pub fn new() -> Self {
        Self {
            nack: HashSet::new(),
            ack: HashMap::new(),
            ack_counts: HashSet::new(),
            ordered_channels: HashMap::new(),
            fragmented_frames: HashMap::new(),
            recv_win_start: 0,
            recv_win_end: 0,
            recv_win_high: 0,
        }
    }

    pub fn handle(
        &mut self,
        connection: &mut Connection,
        payload: &[u8],
    ) -> Result<(), RakHandlerError> {
        // first get the id of the packet.
        let maybe_id = payload.get(0);

        if !maybe_id.is_some() {
            return Ok(());
        }

        let id = maybe_id.unwrap();

        match id {
            0x80..=0x8d => {
                // this is a frame packet
                self.handle_frame(connection, payload);
            }
            0xa0 => {
                // this is an NACK packet, we need to send this packet back!
                // let's check to see if we even have this packet.
                let nack = Ack::compose(payload, &mut 0)?;

                // check the records
                for record in nack.records {
                    match record {
                        Record::Single(rec) => {
                            // we're looking for a single record.
                            if self.ack.contains_key(&rec.sequence) {
                                // lets clone the buffer and send it back.
                                let buffer = self.ack.get(&rec.sequence).unwrap().clone();
                                connection.send(buffer, true);
                                self.ack.remove(&rec.sequence);
                            }
                            // We don't have this record, but there's nothing we can do about it.
                            return Ok(());
                        }
                        Record::Range(rec) => {
                            // we're looking for a range of records.
                            // we need to check if we have any of the records in the range.
                            // we'll check the ack map for each record in the range.
                            for i in rec.start..rec.end {
                                if self.ack.contains_key(&i) {
                                    // we have this record, lets clone the buffer and send it back.
                                    let buffer = self.ack.get(&i).unwrap().clone();
                                    connection.send(buffer, false);
                                    self.ack.remove(&i);
                                }
                            }
                            // We don't have this record, but there's nothing we can do about it.
                            return Ok(());
                        }
                    }
                }
            }
            0xa1 => {
                // this is an ACK packet, we can remove the packet from the ACK list (for real).
                let ack = Ack::compose(payload, &mut 0)?;

                for record in ack.records {
                    match record {
                        Record::Single(rec) => {
                            // we're looking for a single record.
                            self.ack.remove(&rec.sequence);
                            self.ack_counts.remove(&rec.sequence);
                        }
                        Record::Range(rec) => {
                            // we're looking for a range of records.
                            // we need to check if we have any of the records in the range.
                            // we'll check the ack map for each record in the range.
                            for i in rec.start..rec.end {
                                self.ack.remove(&i);
                                self.ack_counts.remove(&i);
                            }
                        }
                    }
                }

                return Ok(());
            }
            _ => {
                // this is an unknown packet, we need to send this packet back!
                return Ok(());
            }
        }

        Ok(())
    }

    fn handle_frame(
        &mut self,
        connection: &mut Connection,
        payload: &[u8],
    ) -> Result<(), RakHandlerError> {
        let mut frame = FramePacket::compose(payload, &mut 1)?;
        // get sequence
        if self.nack.contains(&frame.sequence) {
            self.ack_counts.insert(frame.sequence.clone());
        }

        Ok(())
    }
}
