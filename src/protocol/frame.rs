use std::{
    collections::HashMap,
    io::{Cursor, Write},
};

use binary_utils::error::BinaryError;
use binary_utils::*;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};

/// The information for the given fragment.
/// This is used to determine how to reassemble the frame.
#[derive(Debug, Clone)]
pub struct FragmentMeta {
    /// The total number of fragments in this frame.
    pub(crate) size: u32,
    /// The identifier for this fragment.
    /// This is used similar to a ordered channel, where the trailing buffer
    /// will be stored with this identifier.
    pub(crate) id: u16,
    /// The index of the fragment.
    /// This is the arrangement of the fragments in the frame.
    pub(crate) index: u32,
}

use super::reliability::Reliability;

/// Frames are a encapsulation of a packet or packets.
/// They are used to send packets to the connection in a reliable way.
#[derive(Debug, Clone)]
pub struct FramePacket {
    /// The sequence of this frame.
    /// We'll use this to respond with Ack and Nack to.
    /// This is sized check to 24 bits.
    pub sequence: u32,

    /// The frames for this frame packet, not to exceed the mtu size.
    pub frames: Vec<Frame>,

    /// This is internal use only.
    pub(crate) reliability: Reliability,

    /// This is internal use only.
    pub(crate) byte_length: usize,
}

impl FramePacket {
    /// Creates an empty frame packet.
    pub fn new() -> Self {
        Self {
            sequence: 0,
            frames: Vec::new(),
            reliability: Reliability::ReliableOrd,
            byte_length: 0,
        }
    }

    /// Paritions a stream into a bunch of fragments and returns a frame packet
    /// that is partitioned, otherwise known as "fragmented".
    /// This does not modify reliability. That is up to the caller.
    pub fn partition(stream: Vec<u8>, id: u16, frag_size: u32) -> Vec<Frame> {
        let mut meta: FragmentMeta = FragmentMeta {
            size: 0,
            id,
            index: 0,
        };

        let mut frames: Vec<Frame> = Vec::new();
        let mut position: usize = 0;

        while position < stream.len() {
            // check whether or not we can read the rest of of the stream
            if stream[position..].len() < frag_size as usize {
                // we can reliably read the rest of the buffer into a single frame.
                let mut frame = Frame::init();
                frame.body = stream[position..].to_vec();
                frame.fragment_meta = Some(meta.clone());
                frames.push(frame);
                break;
            } else {
                // we can't read the rest of the stream into a single frame
                // continue to split into multiple frames.
                let mut frame = Frame::init();
                let to_pos = position + (frag_size as usize);
                frame.body = stream[position..to_pos].to_vec();
                frame.fragment_meta = Some(meta.clone());
                frames.push(frame);
                position = to_pos;
                meta.index += 1;
            }
        }

        meta.size = frames.len() as u32;

        // Let's fix up the meta data in each frame.
        for frame in frames.iter_mut() {
            if let Some(m) = frame.fragment_meta.as_mut() {
                m.size = meta.size;
            }
        }

        return frames;
    }
}

impl Streamable for FramePacket {
    fn compose(source: &[u8], position: &mut usize) -> Result<Self, error::BinaryError> {
        let mut stream = Cursor::new(source);
        stream.set_position(*position as u64);
        stream.read_u8()?;
        let mut frames: Vec<Frame> = Vec::new();
        let sequence = stream.read_u24::<LittleEndian>()?;
        let mut offset: usize = stream.position() as usize;

        loop {
            if stream.position() > source.len() as u64 {
                return Ok(FramePacket {
                    reliability: Reliability::ReliableOrd,
                    sequence,
                    frames,
                    byte_length: 0,
                });
            }

            if stream.position() == source.len() as u64 {
                break Ok(FramePacket {
                    reliability: Reliability::ReliableOrd,
                    sequence,
                    frames,
                    byte_length: 0,
                });
            }

            if let Ok(frame) = Frame::compose(&source, &mut offset) {
                stream.set_position(offset as u64);
                frames.push(frame.clone());

                // if frame.parse()?.len() + stream.position() as usize > source.len() {
                //     return Ok(FramePacket { sequence, frames, byte_length: 0 });
                // } else {
                //     continue;
                // }
            } else {
                return Err(BinaryError::RecoverableKnown(
                    "Frame composition failed! Failed to read frame.".into(),
                ));
            }
        }
    }

    fn parse(&self) -> Result<Vec<u8>, BinaryError> {
        let mut stream = Cursor::new(Vec::new());
        stream.write_u8(0x80)?;
        stream.write_u24::<LittleEndian>(self.sequence)?;

        for frame in &self.frames {
            stream.write_all(&frame.parse()?)?;
        }

        Ok(stream.into_inner())
    }
}

/// An individual data frame, these are constructed from a payload.
#[derive(Debug, Clone)]
pub struct Frame {
    /// The flags for this frame, the first 3 bits are reserved for the reliability while the 4th
    /// bit is used to represent if this is a fragment.
    pub flags: u8,
    /// The length of the body of the frame.
    /// This is sized to 24 bits internally, so any number here must be within that range.
    pub size: u16,
    /// The Reliable index of the frame (if reliable)
    pub reliable_index: Option<u32>,
    /// The sequenced index of the frame (if sequenced)
    /// This is used to determine the position in frame list.
    pub sequence_index: Option<u32>,
    /// The order index of the frame (if ordered)
    /// This is used to determine the position in frame list,
    /// This is different from the sequence index in that it is
    /// used more to sequence packets in a specific manner.
    pub order_index: Option<u32>,
    /// The order channel of the frame (if ordered)
    /// This is used to store order information for the frame.
    pub order_channel: Option<u8>,
    /// The information for fragmentation (if the frame is split into parts)
    /// This is used to determine how to reassemble the frame.
    pub fragment_meta: Option<FragmentMeta>,
    /// The reliability of this frame, this is essentially used to save frames and send them back if
    /// they are lost. Otherwise, the frame is sent unreliably.
    pub reliability: Reliability,
    /// The body of the frame, this is the payload of the frame.
    pub body: Vec<u8>,
}

impl Frame {
    /// Initializes a new empty frame that is Unreliable.
    /// This is usually used to inject data into.
    pub fn init() -> Self {
        Self {
            flags: 0,
            size: 0,
            reliable_index: None,
            sequence_index: None,
            order_index: None,
            order_channel: None,
            fragment_meta: None,
            reliability: Reliability::Unreliable,
            body: Vec::new(),
        }
    }

    /// Whether or not the frame is fragmented.
    pub fn is_fragmented(&self) -> bool {
        self.fragment_meta.is_some()
    }

    /// Whether or not the frame is sequenced and reliable.
    pub fn is_sequenced(&self) -> bool {
        self.reliability.is_sequenced()
    }
}

impl Streamable for Frame {
    fn compose(source: &[u8], position: &mut usize) -> Result<Self, error::BinaryError> {
        let mut stream = Cursor::new(source.to_vec());

        // create a dummy frame for us to write to.
        let mut frame: Frame = Frame::init();

        // set the position to the current position
        stream.set_position(*position as u64);

        // read the flags
        frame.flags = stream.read_u8()?;
        // set the reliability
        frame.reliability = Reliability::from_flags(frame.flags);

        // read the length of the body in bits
        frame.size = stream.read_u16::<BigEndian>()? / 8;

        // check whether or not this frame is reliable, if it is, read the reliable index
        if frame.reliability.is_reliable() {
            frame.reliable_index = Some(stream.read_u24::<LittleEndian>()?);
        }

        // check whether or not this frame is sequenced, if it is, read the sequenced index
        if frame.reliability.is_sequenced() {
            frame.sequence_index = Some(stream.read_u24::<LittleEndian>()?);
        }

        // check whether or not this frame is ordered, if it is, read the order index
        // and order channel
        if frame.reliability.is_sequenced_or_ordered() {
            frame.order_index = Some(stream.read_u24::<LittleEndian>()?);
            frame.order_channel = Some(stream.read_u8()?);
        }

        // check whether or not this frame is fragmented, if it is, read the fragment meta
        if (frame.flags & 0x10) > 0 {
            frame.fragment_meta = Some(FragmentMeta {
                size: stream.read_u32::<BigEndian>()?.try_into().unwrap(),
                id: stream.read_u16::<BigEndian>()?,
                index: stream.read_u32::<BigEndian>()?.try_into().unwrap(),
            });
        }

        // read the body
        frame.body = (&source
            [stream.position() as usize..stream.position() as usize + frame.size as usize])
            .to_vec();
        // update the position.
        *position = stream.position() as usize + frame.size as usize;

        Ok(frame)
    }

    fn parse(&self) -> Result<Vec<u8>, error::BinaryError> {
        let mut stream = Cursor::new(Vec::new());
        // generate the flags!
        let mut flags = self.reliability.to_flags();

        // check whether or not this frame is fragmented, if it is, set the fragment flag
        if self.fragment_meta.is_some() {
            flags |= 0x10;
        }

        let size = self.body.len() as u16;

        // write the flags
        stream.write_u8(flags)?;
        // write the length of the body in bits
        stream.write_u16::<BigEndian>(size * 8)?;

        // check whether or not this frame is reliable, if it is, write the reliable index
        if self.reliability.is_reliable() {
            stream.write_u24::<LittleEndian>(self.reliable_index.unwrap())?;
        }

        // check whether or not this frame is sequenced, if it is, write the sequenced index
        if self.reliability.is_sequenced() {
            stream.write_u24::<LittleEndian>(self.sequence_index.unwrap())?;
        }

        // check whether or not this frame is ordered, if it is, write the order index
        // and order channel
        if self.reliability.is_sequenced_or_ordered() {
            stream.write_u24::<LittleEndian>(self.order_index.unwrap())?;
            stream.write_u8(self.order_channel.unwrap())?;
        }

        // check whether or not this frame is fragmented, if it is, write the fragment meta
        if self.fragment_meta.is_some() {
            let fragment_meta = self.fragment_meta.as_ref().unwrap();
            stream.write_u32::<BigEndian>(fragment_meta.size.try_into().unwrap())?;
            stream.write_u16::<BigEndian>(fragment_meta.id)?;
            stream.write_u32::<BigEndian>(fragment_meta.index.try_into().unwrap())?;
        }

        // write the body
        stream.write_all(&self.body)?;

        Ok(stream.get_ref().clone())
    }
}
