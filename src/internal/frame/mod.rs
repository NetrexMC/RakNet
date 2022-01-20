pub mod fragment;
pub mod reliability;

use std::io::{Cursor, Read, Write};

use binary_utils::*;
use binary_utils::u24;
use byteorder::{ReadBytesExt, LittleEndian, BigEndian, WriteBytesExt};

use self::fragment::FragmentMeta;
use self::reliability::Reliability;

/// Frames are a encapsulation of a packet or packets.
/// They are used to send packets to the connection in a reliable way.
pub struct FramePacket {
    /// The sequence of this frame.
    /// We'll use this to respond with Ack and Nack to.
    /// This is sized check to 24 bits.
    pub sequence: u32,

    /// The frames for this frame packet, not to exceed the mtu size.
    pub frames: Vec<Frame>,
}

/// An individual data frame, these are constructed from a payload.
pub struct Frame {
    /// The sequence number for the frame, this is a triad.
    pub sequence: u64,
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
            sequence: 0,
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
        if frame.reliability.is_ordered() {
            frame.order_index = Some(stream.read_u24::<LittleEndian>()?);
            frame.order_channel = Some(stream.read_u8()?);
        }

        // check whether or not this frame is fragmented, if it is, read the fragment meta
        if (frame.flags & 0x10) > 0 {
            frame.fragment_meta = Some(FragmentMeta {
                size: stream.read_u24::<BigEndian>()?.try_into().unwrap(),
                id: stream.read_u16::<BigEndian>()?,
                index: stream.read_u24::<BigEndian>()?.try_into().unwrap(),
            });
        }

        if source.len() > (frame.size as usize) {
            // read the body
            let offset = stream.position() as usize;
            let inner_buffer = &source[offset..(frame.size as usize) + offset];
            stream.set_position(stream.position() + (frame.size as u64));
            frame.body = inner_buffer.to_vec();
        }

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

        // write the flags
        stream.write_u8(flags)?;
        // write the length of the body in bits
        stream.write_u16::<BigEndian>(self.size * 8)?;

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
        if self.reliability.is_ordered() {
            stream.write_u24::<LittleEndian>(self.order_index.unwrap())?;
            stream.write_u8(self.order_channel.unwrap())?;
        }

        // check whether or not this frame is fragmented, if it is, write the fragment meta
        if self.fragment_meta.is_some() {
            let fragment_meta = self.fragment_meta.as_ref().unwrap();
            stream.write_u24::<BigEndian>(fragment_meta.size.try_into().unwrap())?;
            stream.write_u16::<BigEndian>(fragment_meta.id)?;
            stream.write_u24::<BigEndian>(fragment_meta.index.try_into().unwrap())?;
        }

        // write the body
        stream.write_all(&self.body)?;

        Ok(stream.get_ref().clone())
    }
}