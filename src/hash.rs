use sha2::{Digest, Sha256};
use std::fmt;
use std::io::Write;
use std::hash;

/// Identifier for an object.
///
/// Because they are content-addressable, this is a hash of its content.
pub struct ID {
    pub bytes: [u8; 32],
}

impl ID {
    pub fn from_slice(buf: &[u8]) -> Option<ID> {
        if buf.len() == 32 {
            let mut bytes = [0u8; 32];
            bytes.clone_from_slice(buf);
            Some(ID { bytes: bytes })
        } else {
            None
        }
    }

    pub fn hash_size() -> usize {
        32
    }
}

impl Eq for ID {}

impl hash::Hash for ID {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

impl fmt::Display for ID {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for byte in &self.bytes {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

/// Content to ID code.
///
/// Abstracted to make it easier to swap it out, or use multiple hashes,
/// but there is no current plan to make the lib generic on this.
pub struct Hasher {
    hasher: Sha256,
}

impl Hasher {
    pub fn new() -> Hasher {
        Hasher { hasher: Sha256::new() }
    }

    pub fn update(&mut self, msg: &[u8]) {
        self.hasher.input(msg);
    }

    pub fn result(self) -> ID {
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(self.hasher.result().as_slice());
        ID { bytes: bytes }
    }
}
