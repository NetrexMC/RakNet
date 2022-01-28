use binary_utils::{error::BinaryError, Streamable};

use crate::MAGIC;

#[derive(Debug, Clone)]
pub struct Magic(pub Vec<u8>);

impl Magic {
    pub fn new() -> Self {
        Self(MAGIC.to_vec())
    }
}

impl Streamable for Magic {
    fn parse(&self) -> Result<Vec<u8>, BinaryError> {
        Ok(MAGIC.to_vec())
    }

    fn compose(source: &[u8], position: &mut usize) -> Result<Self, BinaryError> {
        // magic is 16 bytes
        let pos = *position + (16 as usize);
        let magic = &source[*position..pos];
        *position += 16;

        if magic.to_vec() != MAGIC.to_vec() {
            Err(BinaryError::RecoverableKnown(
                "Could not construct magic from malformed bytes.".to_string(),
            ))
        } else {
            Ok(Self(magic.to_vec()))
        }
    }
}
