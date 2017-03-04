use sha2::{Digest, Sha256};
use std::fmt;
use std::io::{self, Write};
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

    pub fn hex(&self) -> String {
        let mut hex = Vec::with_capacity(Self::hash_size() * 2);
        for byte in &self.bytes {
            write!(&mut hex, "{:02x}", byte).unwrap();
        }
        unsafe { String::from_utf8_unchecked(hex) }
    }

    pub fn from_hex(hex: &[u8]) -> Option<ID> {
        if hex.len() != Self::hash_size() * 2 {
            return None;
        }
        let mut modulus = 0;
        let mut buf = 0u8;
        let mut out = [0u8; 32];
        for (i, byte) in hex.iter().cloned().enumerate() {
            buf <<= 4;

            match byte {
                b'0'...b'9' => buf |= byte - b'0',
                b'a'...b'f' => buf |= byte - b'a' + 10,
                // Also accept uppercase, though we don't write it
                b'A'...b'F' => buf |= byte - b'A' + 10,
                _ => return None,
            }

            modulus += 1;
            if modulus == 2 {
                modulus = 0;
                out[i / 2] = buf;
            }
        }
        Some(ID { bytes: out })
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

    pub fn result(self) -> ID {
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(self.hasher.result().as_slice());
        ID { bytes: bytes }
    }
}

impl Write for Hasher {
    fn write(&mut self, msg: &[u8]) -> io::Result<usize> {
        self.hasher.input(msg);
        Ok(msg.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
