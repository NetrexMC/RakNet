use binary_util::interfaces::{Reader, Writer};
use binary_util::io::{ByteReader, ByteWriter};

/// A unique identifier recoginzing the client as offline.
pub(crate) const MAGIC: [u8; 16] = [
    0x00, 0xff, 0xff, 0x0, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78,
];

/// The magic packet is sent to the server to identify the client as offline.
/// This is a special raknet header that uniquely identifies the protocol as raknet.
#[derive(Debug, Clone)]
pub struct Magic;

impl Magic {
    pub fn new() -> Self {
        Self {}
    }
}

impl Reader<Magic> for Magic {
    fn read(buf: &mut ByteReader) -> Result<Magic, std::io::Error> {
        let mut magic = [0u8; 16];
        buf.read(&mut magic)?;

        if magic != MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid magic",
            ));
        }

        Ok(Magic)
    }
}

impl Writer for Magic {
    fn write(&self, buf: &mut ByteWriter) -> Result<(), std::io::Error> {
        buf.write(&MAGIC)?;
        Ok(())
    }
}
