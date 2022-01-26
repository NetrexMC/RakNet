use binary_utils::*;
use std::{
    collections::{HashMap, HashSet},
    fmt,
    io::Write,
};

use crate::connection::Connection;

use super::{
    ack::{Ack, Record},
    frame::{reliability::Reliability, Frame, FramePacket},
    queue::OrderedQueue,
};

#[derive(Debug)]
pub enum RakHandlerError {
    Unknown(String),
    BinaryError(binary_utils::error::BinaryError),
    UnknownPacket(u8),
}

impl fmt::Display for RakHandlerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RakHandlerError::Unknown(s) => write!(f, "Unknown error: {}", s),
            RakHandlerError::BinaryError(e) => write!(f, "Binary error: {:?}", e),
            RakHandlerError::UnknownPacket(p) => write!(f, "Unknown packet: {}", p),
        }
    }
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
#[derive(Debug, Clone)]
pub struct RakConnHandlerMeta {
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
    pub ordered_channels: OrderedQueue<Vec<u8>>,
    /// The fragmented frames that are waiting for reassembly.
    pub fragmented_frames: HashMap<u16, HashMap<u32, Frame>>,
    /// The sequence number used to send packets.
    /// This is incremented every time we send a packet.
    pub send_seq: u32,
    /// The next order index to use for ordered channels.
    /// This is incremented every time we send a packet with an order channel.
    pub order_index: u32,
    /// The next message index, this is basically each reliable message.
    /// This is incremented every time we send a packet with a reliable channel.
    pub message_index: u32,
    /// The last sent sequence number.
    pub ack_order_index: u32,
    /// The fragment id to be used next.
    pub fragment_ids: HashSet<u16>,
}

impl RakConnHandlerMeta {
    pub fn new() -> Self {
        Self {
            nack: HashSet::new(),
            ack: HashMap::new(),
            ack_counts: HashSet::new(),
            ordered_channels: OrderedQueue::new(),
            fragmented_frames: HashMap::new(),
            send_seq: 0,
            order_index: 0,
            message_index: 0,
            ack_order_index: 0,
            fragment_ids: HashSet::new(),
        }
    }

    pub fn next_seq(&mut self) -> u32 {
        self.send_seq += 1;
        self.send_seq
    }

    pub fn next_order_index(&mut self) -> u32 {
        self.order_index += 1;
        self.order_index
    }

    pub fn next_message_index(&mut self) -> u32 {
        self.message_index += 1;
        self.message_index
    }

    pub fn next_order_ack_index(&mut self) -> u32 {
        self.ack_order_index += 1;
        self.ack_order_index
    }

    pub fn next_fragment_id(&mut self) -> u16 {
        let next = self.fragment_ids.len() as u16;
        self.fragment_ids.insert(next);
        next
    }

    pub fn free_fragment_id(&mut self, id: u16) {
        self.fragment_ids.remove(&id);
    }
}

/// This is hacked struct to allow mutability across the handler.
/// Not sure if this is the best way to do this but it will stay for now.
#[derive(Debug, Clone)]
pub struct RakConnHandler;

impl RakConnHandler {
    pub fn handle(connection: &mut Connection, payload: &[u8]) -> Result<(), RakHandlerError> {
        // first get the id of the packet.
        let maybe_id = payload.get(0);

        if !maybe_id.is_some() {
            return Ok(());
        }

        let id = maybe_id.unwrap();

        match id {
            0x80..=0x8d => {
                // this is a frame packet
                return Self::handle_frame(connection, payload);
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
                            if connection.rakhandler.ack.contains_key(&rec.sequence) {
                                // lets clone the buffer and send it back.
                                let buffer = connection
                                    .rakhandler
                                    .ack
                                    .get(&rec.sequence)
                                    .unwrap()
                                    .clone();
                                connection.send(buffer, true);
                                connection.rakhandler.ack.remove(&rec.sequence);
                                connection.rakhandler.ack_counts.remove(&rec.sequence);
                            }
                            // We don't have this record, but there's nothing we can do about it.
                            return Ok(());
                        }
                        Record::Range(mut rec) => {
                            rec.fix();
                            // we're looking for a range of records.
                            // we need to check if we have any of the records in the range.
                            // we'll check the ack map for each record in the range.
                            for i in rec.start..rec.end {
                                if connection.rakhandler.ack.contains_key(&i) {
                                    // we have this record, lets clone the buffer and send it back.
                                    let buffer = connection.rakhandler.ack.get(&i).unwrap().clone();
                                    connection.send(buffer, true);
                                    connection.rakhandler.ack.remove(&i);
                                }
                            }
                            // We don't have this record, but there's nothing we can do about it.
                            return Ok(());
                        }
                    }
                }

                return Ok(());
            }
            0xc0 => {
                // this is an ACK packet from the client, we can remove the packet from the ACK list (for real).
                let ack = Ack::compose(payload, &mut 0)?;

                for record in ack.records {
                    match record {
                        Record::Single(rec) => {
                            // we're looking for a single record.
                            connection.rakhandler.nack.remove(&rec.sequence);
                            // connection.rakhandler.ack_counts.remove(&rec.sequence);
                        }
                        Record::Range(mut rec) => {
                            rec.fix();
                            // we're looking for a range of records.
                            // we need to check if we have any of the records in the range.
                            // we'll check the ack map for each record in the range.
                            for i in rec.start..rec.end {
                                connection.rakhandler.nack.remove(&i);
                                // connection.rakhandler.ack_counts.remove(&i);
                            }
                        }
                    }
                }

                return Ok(());
            }
            _ => {
                // this is an unknown packet, we don't know what to do with it.
                return Err(RakHandlerError::UnknownPacket(*id));
            }
        }
    }

    fn handle_frame(connection: &mut Connection, payload: &[u8]) -> Result<(), RakHandlerError> {
        let frame_packet = FramePacket::compose(&payload, &mut 0)?;

        // let's handle each individual frame of the packet
        for frame in frame_packet.frames {
            if frame.reliability.is_reliable() {
                connection
                    .rakhandler
                    .ack_counts
                    .insert(frame_packet.sequence);
            }
            if frame.is_fragmented() {
                // The fragmented frame meta data.
                let meta = frame.fragment_meta.as_ref().unwrap();
                // The fragmented frames bounded by this id.
                let parts = connection
                    .rakhandler
                    .fragmented_frames
                    .entry(meta.id)
                    .or_insert(HashMap::new());

                // We need to check if we have all the parts of the frame.
                // If we do, we'll reassemble the frame.
                if parts.len() != meta.size as usize {
                    // We don't have all the parts, lets add this part to the list.
                    parts.insert(meta.index, frame.clone());
                }

                if parts.len() == meta.size as usize {
                    // We have all the fragments, we can reassemble the frame.
                    // Sense we need to order this by their index, we need to sort the parts.
                    let mut parts = parts.iter().collect::<Vec<_>>();
                    parts.sort_by_key(|f| f.0);

                    // our parts are now sorted, we can now reassemble the frame.
                    let mut buffer = Vec::new();
                    for (_, frm) in parts {
                        buffer.write_all(&frm.body).unwrap();
                    }

                    // This is now an online packet! we can handle it.
                    // make a fake frame now.
                    let mut fake_frame = frame.clone();
                    fake_frame.body = buffer;
                    fake_frame.fragment_meta = None;

                    Self::handle_entire_frame(connection, fake_frame.clone())?;
                }
            } else {
                Self::handle_entire_frame(connection, frame.clone())?;
            }
        }

        Ok(())
    }

    fn handle_entire_frame(
        connection: &mut Connection,
        frame: Frame,
    ) -> Result<(), RakHandlerError> {
        if frame.is_sequenced() || frame.reliability.is_reliable() {
            if frame.reliability.is_ordered() {
                // todo: Actually handle order
                let id = frame.order_index.unwrap();
                let success = connection
                    .rakhandler
                    .ordered_channels
                    .insert(frame.body.clone(), id);
                if success {
                    Self::handle_packet(connection, frame.body)?;
                } else {
                    // this is an old or duplicated packet!
                    #[cfg(feature = "debug")]
                    rak_debug!("Duplicate packet! {:?}", frame);
                }
            } else {
                // todo the frame is sequenced and reliable, we can handle it.
                // todo remove this hack and actually handle the sequence!
                Self::handle_packet(connection, frame.body)?;
            }
        } else {
            Self::handle_packet(connection, frame.body)?;
        }

        Ok(())
    }

    fn handle_packet(connection: &mut Connection, packet: Vec<u8>) -> Result<(), RakHandlerError> {
        // first try to handle the packet.
        // let packet = Packet::compose(&packet, &mut 0)?;
        // we should check ack here.
        if packet.len() == 0 {
            return Ok(());
        }
        if packet[0] == 0xa0 || packet[0] == 0xc0 {
            // this is an ack packet, we need to re-handle this.
            Self::handle(connection, &packet)?;
        } else {
            connection.handle(packet);
        }
        Ok(())
    }

    /// Makes a frame packet with the given payload, this is used to directly send a packet
    /// to the client.
    ///
    /// In the future we should allow use of custom reliability, but for now, we'll be sending everything from raknet unreliably.
    pub fn create_frame(
        connection: &mut Connection,
        payload: Vec<u8>,
    ) -> Result<FramePacket, RakHandlerError> {
        // update sequence.
        let mut frame_packet = FramePacket {
            sequence: connection.rakhandler.next_seq(),
            frames: Vec::new(),
            byte_length: 0,
        };

        // todo: Check the reliability kind.
        let mut frame = Frame::init();
        frame.reliability = Reliability::Unreliable;
        frame.body = payload;

        frame_packet.frames.push(frame);

        Ok(frame_packet)
    }

    /// This function will send all the frames given to the client without exceeding the MTU size!
    fn send_frames(connection: &mut Connection, mut frames: Vec<Frame>, ack: bool) {
        // this will send each frame in it's own packet. if it's a fragmented.

        if frames.len() == 0 {
            return;
        }

        // get the frames that are free now.
        let mut sent: HashMap<u16, (u32, Vec<u32>)> = HashMap::new();
        let mut free: Vec<u16> = Vec::new();

        let mut outbound = FramePacket::new();
        outbound.sequence = connection.rakhandler.next_seq();

        for frame in frames.iter_mut() {
            // todo: FIX ReliableOrdered
            frame.reliability = Reliability::Unreliable;
            if ack {
                // frame.reliability = Reliability::ReliableOrdAck;
                // frame.reliable_index = Some(connection.rakhandler.next_message_index());
                // frame.order_channel = Some(1);
                // frame.order_index = Some(connection.rakhandler.next_order_ack_index());
            } else {
                // frame.reliability = Reliability::ReliableOrd;
                // frame.reliable_index = Some(connection.rakhandler.next_message_index());
                // frame.order_channel = Some(0);
                // frame.order_index = Some(connection.rakhandler.next_order_index());
            }

            if frame.is_fragmented() {
                if let Some(meta) = frame.fragment_meta.clone() {
                    // we need to free this fragment id if we've sent all the parts.
                    let parts = sent.entry(meta.id).or_insert((meta.size, Vec::new()));
                    parts.1.push(meta.index);

                    if parts.1.len() == parts.0 as usize {
                        // we've sent all the parts, we can free this id.
                        sent.remove(&meta.id);
                        free.push(meta.id);
                    }
                }
            }

            if frame.fparse().len() + outbound.byte_length > (connection.mtu - 60).into() {
                // we need to send this packet.
                connection.send_immediate(outbound.fparse());
                outbound = FramePacket::new();
                outbound.sequence = connection.rakhandler.next_seq();
            } else {
                outbound.frames.push(frame.clone());
            }
        }

        // send the last packet.
        connection.send_immediate(outbound.fparse());

        for id in free {
            connection.rakhandler.free_fragment_id(id);
        }
    }

    /// This is an instant send, this will send the packet to the client immediately.
    pub fn send_as_framed(connection: &mut Connection, payload: Vec<u8>) {
        if payload.len() < 60 || (payload.len() - 60) < connection.mtu.into() {
            let mut frame = Frame::init();
            frame.body = payload;
            Self::send_frames(connection, vec![frame], false);
        } else {
            let frames = FramePacket::partition(
                payload,
                connection.rakhandler.next_fragment_id(),
                (connection.mtu - 60).into(),
            );
            Self::send_frames(connection, frames, false);
        }
    }

    pub fn tick(connection: &mut Connection) {
        // lets send the packets in the queue now.
        let packets = connection.queue.flush();
        let mut current_frame_id: u16 = 0;

        for packet in packets {
            // we need to handle these packets!
            let mut frames =
                FramePacket::partition(packet, current_frame_id, (connection.mtu - 60).into());
            for frame in frames.iter_mut() {
                if frame.is_fragmented() {
                    if let Some(meta) = frame.fragment_meta.as_mut() {
                        meta.id = current_frame_id;
                    }
                }
            }
            current_frame_id += 1;
            Self::send_frames(connection, frames, false);
        }

        if connection.state.is_connected() {
            // send the acks to the client that we got some packets

            // // get missing packets and request them.
            let missing = connection.rakhandler.ordered_channels.flush_missing();

            if missing.len() != 0 {
                let nack = Ack::from_missing(missing);

                #[cfg(feature = "debug")]
                rak_debug!("NACK: {:#?}", nack);

                connection.send(nack.fparse(), true);
            }

            let mut ack = Ack::new(connection.rakhandler.ack_counts.len() as u16, false);
            for id in connection.rakhandler.ack_counts.iter() {
                ack.push_record(*id);
            }

            if ack.records.len() != 0 {
                connection.rakhandler.ack_counts.clear();
                connection.send(ack.fparse(), true);
            }
        }
    }
}
