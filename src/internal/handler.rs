use binary_utils::*;
use std::{
    collections::{HashMap, HashSet},
    fmt,
    io::Write,
};

use crate::connection::Connection;

use super::{
    ack::{Ack, Record},
    frame::{
        reliability::{cache::CacheStore, Reliability},
        Frame, FramePacket,
    },
    queue::OrderedQueue,
};

#[cfg(feature = "debug")]
use crate::rak_debug;

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
    pub ack: CacheStore<u32, Vec<u8>>,
    /// A queue to send back to the client to acknowledge we've recieved these packets.
    pub ack_counts: HashSet<u32>,
    /// The ordered channels that have been recieved and are waiting for completion.
    /// Ordered channels will be reorded once all the packets have been received.
    pub ordered_channels: OrderedQueue<Vec<u8>>,
    /// The fragmented frames that are waiting for reassembly.
    pub fragmented_frames: HashMap<u16, HashMap<u32, Frame>>,
    /// The sequence number used to send packets.
    /// This is incremented every time we send a packet that is reliable.
    /// Any packets that are reliable, can be re-sent if they are acked.
    pub send_seq: u32,
    /// The next order index to use for ordered channels.
    /// This is incremented every time we send a packet with an order channel.
    pub order_index: HashMap<u8, u32>,
    /// The next sequence index to use for ordered channels & sequenced packets.
    /// Each sequence varies on the order channel.
    /// This is incremented every time we send a packet with an order channel.
    pub seq_index: HashMap<u8, u32>,
    /// The next message index, this is basically each reliable message.
    /// This is incremented every time we send a packet with a reliable channel.
    pub message_index: HashMap<i16, u32>,
    /// The fragment id to be used next.
    pub fragment_ids: HashSet<u16>,
}

impl RakConnHandlerMeta {
    pub fn new() -> Self {
        Self {
            nack: HashSet::new(),
            ack: CacheStore::new(),
            ack_counts: HashSet::new(),
            ordered_channels: OrderedQueue::new(),
            fragmented_frames: HashMap::new(),
            send_seq: 0,
            order_index: HashMap::new(),
            message_index: HashMap::new(),
            seq_index: HashMap::new(),
            fragment_ids: HashSet::new(),
        }
    }

    pub fn next_seq(&mut self) -> u32 {
        self.send_seq += 1;
        self.send_seq
    }

    pub fn get_order_index(&mut self, channel: u8) -> u32 {
        *self.order_index.entry(channel).or_insert(0)
    }

    pub fn next_order_index(&mut self, channel: u8) -> u32 {
        let index = self.order_index.entry(channel).or_insert(0);
        let cpy = *index;
        *index += 1;
        return cpy;
    }

    #[allow(dead_code)]
    pub fn get_reliable_index(&mut self, channel: u8) -> u32 {
        *self.message_index.entry(channel.into()).or_insert(0)
    }

    pub fn next_reliable_index(&mut self, channel: u8) -> u32 {
        let index = self.message_index.entry(channel.into()).or_insert(0);
        let cpy = *index;
        *index += 1;
        return cpy;
    }

    #[allow(dead_code)]
    pub fn get_sequence_index(&mut self, channel: u8) -> u32 {
        *self.seq_index.entry(channel).or_insert(0)
    }

    pub fn next_sequence_index(&mut self, channel: u8) -> u32 {
        let index = self.seq_index.entry(channel).or_insert(0);
        let cpy = *index;
        *index += 1;
        return cpy;
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
    /// Handles the raw payload from the connection (without the header).
    /// This will check the header and then handle the packet according to that header.
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
                return Self::handle_raw_frame(connection, payload);
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
                            if connection.rakhandler.ack.has(&rec.sequence) {
                                // flush the cache for only this sequence
                                if let Some(packets) =
                                    connection.rakhandler.ack.flush_key(rec.sequence)
                                {
                                    for packet in packets.1 {
                                        connection.send(packet, true);
                                    }
                                    connection.rakhandler.ack_counts.remove(&rec.sequence);
                                }
                            }
                            // We don't have this record, but there's nothing we can do about it.
                        }
                        Record::Range(mut rec) => {
                            rec.fix();
                            // we're looking for a range of records.
                            // we need to check if we have any of the records in the range.
                            // we'll check the ack map for each record in the range.
                            for i in rec.start..rec.end {
                                if connection.rakhandler.ack.has(&i) {
                                    // flush the cache for only this sequence
                                    if let Some(packets) = connection.rakhandler.ack.flush_key(i) {
                                        for packet in packets.1 {
                                            connection.send(packet, true);
                                        }
                                        connection.rakhandler.ack_counts.remove(&i);
                                    }
                                }
                            }
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

    /// Handles a raw frame packet.
    /// This packet has not yet been validated nor constructed,
    /// this method will parse and validate the packet as well as performing
    /// necessary reliability actions.
    fn handle_raw_frame(
        connection: &mut Connection,
        payload: &[u8],
    ) -> Result<(), RakHandlerError> {
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

                    Self::handle_frame(connection, fake_frame.clone())?;
                }
            } else {
                Self::handle_frame(connection, frame.clone())?;
            }
        }

        Ok(())
    }

    /// Handles a single frame within a packet.
    /// This method really only handles the reliability of the packet,
    /// in that, if it is ordered, it will order it as it was sent.
    /// And other related utilities.
    fn handle_frame(connection: &mut Connection, frame: Frame) -> Result<(), RakHandlerError> {
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

    /// Sugar syntax method, does a few validations checks and sends the packet over to the
    /// connection to be handled further.
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

    /// This function will batch all the frames together and send them to the client with the specified
    /// reliability.
    ///
    /// If the packet is unreliable, raknet will not perform any checks to ensure that the client
    /// may request the packet again.
    fn send_frames(connection: &mut Connection, mut frames: Vec<Frame>, reliability: Reliability) {
        // this will send each frame in it's own packet. if it's a fragmented.
        if frames.len() == 0 {
            return;
        }

        // get the frames that are free now.
        let mut sent: HashMap<u16, (u32, Vec<u32>)> = HashMap::new();
        // these are the frames that can be freed from the sent list.
        // this is used to renew fragments so we can have different parts
        let mut free: Vec<u16> = Vec::new();

        let mut order_index: Option<u32> = None;
        let mut sequence: Option<u32> = None;

        // this is an initial check
        if reliability.is_ordered() {
            order_index = Some(connection.rakhandler.next_order_index(0));
        } else if reliability.is_sequenced() {
            // we still need an order index, however we don't need to increase the index.
            order_index = Some(connection.rakhandler.get_order_index(0));
            // increase the sequence for this channel.
            sequence = Some(connection.rakhandler.next_sequence_index(0));
        }

        let mut outbound = FramePacket::new();
        outbound.reliability = reliability;
        outbound.sequence = connection.rakhandler.next_seq();

        // if we need to fragment, then we need to add some complexity, otherwise, we can just send the packet.
        // i have no idea why the fuck this is like this, but it is, lol
        for frame in frames.iter_mut() {
            frame.reliability = reliability;

            if reliability.is_reliable() {
                // this is a reliable frame! Let's write the sequence it's bound to.
                frame.reliable_index = Some(connection.rakhandler.next_reliable_index(0));
            }

            if reliability.is_sequenced() {
                // this is a sequenced frame! Let's write the sequence it's bound to.
                frame.sequence_index = sequence;
            }

            if reliability.is_sequenced_or_ordered() {
                // this is an ordered frame! Let's write the order index it's bound to.
                frame.order_channel = Some(0);
                frame.order_index = Some(order_index.unwrap());
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
                Self::send_frame(connection, &outbound);
                outbound = FramePacket::new();
                outbound.reliability = reliability;
                outbound.sequence = connection.rakhandler.next_seq();
            } else {
                outbound.frames.push(frame.clone());
            }
        }

        // send the last packet.
        Self::send_frame(connection, &outbound);

        for id in free {
            connection.rakhandler.free_fragment_id(id);
        }
    }

    /// This function will send the given frame packet to the client.
    fn send_frame(connection: &mut Connection, frame: &FramePacket) {
        if frame.reliability.is_reliable() {
            // we need to add this to the reliable list.
            // this is buffered and will die if the client doesn't respond.
            let parsed = frame.fparse();
            connection
                .rakhandler
                .ack
                .add(frame.sequence, parsed.clone());
            connection.send_immediate(parsed);
        } else {
            connection.send_immediate(frame.fparse());
        }
    }

    /// This is an instant send, this will send the packet to the client immediately.
    pub fn send_framed(connection: &mut Connection, payload: Vec<u8>, reliability: Reliability) {
        if payload.len() < 60 || (payload.len() - 60) < connection.mtu.into() {
            let mut frame = Frame::init();
            frame.body = payload;
            Self::send_frames(connection, vec![frame], reliability);
        } else {
            let frames = FramePacket::partition(
                payload,
                connection.rakhandler.next_fragment_id(),
                (connection.mtu - 60).into(),
            );
            Self::send_frames(connection, frames, reliability);
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
            Self::send_frames(connection, frames, Reliability::ReliableOrd);
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

            // clear up the packets we've recieved.
            let mut ack = Ack::new(connection.rakhandler.ack_counts.len() as u16, false);
            for id in connection.rakhandler.ack_counts.iter() {
                ack.push_record(*id);
            }

            if ack.records.len() != 0 {
                connection.rakhandler.ack_counts.clear();
                connection.send(ack.fparse(), true);
            }

            // clean up the packets that we need to have an ack for.
            let mut needs_cleared = Vec::<u32>::new();
            for (id, queue) in connection.rakhandler.ack.store.iter() {
                if queue.0.elapsed().unwrap().as_secs() > 5 {
                    needs_cleared.push(*id);
                }
            }
            for id in needs_cleared {
                let packets = connection.rakhandler.ack.flush_key(id).unwrap().1;
                for packet in packets {
                    connection.send_immediate(packet);
                }
            }
        }
    }
}
