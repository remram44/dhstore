//! Structures and functions related to hashing.
//!
//! This module contains the `ID` type used to addres blobs and objects by their
//! content, as well as `Hasher` used to build it from bytes.

use sha2::{Digest, Sha256};
use std::fmt;
use std::io::{self, Write};
use std::hash;

/// Identifier for an object.
///
/// Because they are content-addressable, this is a hash of its content.
#[derive(Clone)]
pub struct ID {
    pub bytes: [u8; 32],
}

impl ID {
    /// Make an ID from raw bytes.
    pub fn from_slice(buf: &[u8]) -> Option<ID> {
        if buf.len() == 32 {
            let mut bytes = [0u8; 32];
            bytes.clone_from_slice(buf);
            Some(ID { bytes: bytes })
        } else {
            None
        }
    }

    /// Returns the size of the hash in bytes.
    ///
    /// This module uses SHA256, therefore this is always 32.
    pub fn hash_size() -> usize {
        32
    }

    /// Returns a string representation of the ID.
    ///
    /// This is the hash in lowercase hexadecimal.
    pub fn hex(&self) -> String {
        let mut hex = Vec::with_capacity(Self::hash_size() * 2);
        for byte in &self.bytes {
            write!(&mut hex, "{:02x}", byte).unwrap();
        }
        unsafe { String::from_utf8_unchecked(hex) }
    }

    /// Parses the string representation into a ID.
    ///
    /// This returns an `ID` if the string was valid, else None.
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

impl PartialEq for ID {
    fn eq(&self, other: &ID) -> bool {
        self.bytes == other.bytes
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

impl fmt::Debug for ID {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "ID({})", self)
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
    /// Build a new `Hasher`.
    ///
    /// Feed it data using the `Write` trait.
    pub fn new() -> Hasher {
        Hasher { hasher: Sha256::new() }
    }

    /// Consume this `Hasher` and return an `ID`.
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

/// A convenient adapter to hash while writing.
///
/// Everything written on this object will be forwarded to the given object,
/// while computing a hash.
pub struct HasherWriter<W: Write> {
    hasher: Hasher,
    writer: W,
}

impl<W: Write> HasherWriter<W> {
    /// Build a new `HasherWrite` that will write on the given object.
    pub fn new(writer: W) -> HasherWriter<W> {
        HasherWriter {
            hasher: Hasher::new(),
            writer: writer,
        }
    }

    /// Consume this object and returns the `ID` computed from hashing.
    ///
    /// The internal `Write` object given at construction is dropped.
    pub fn result(self) -> ID {
        self.hasher.result()
    }
}

impl<W: Write> Write for HasherWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = self.writer.write(buf)?;
        self.hasher.write(&buf[..len]).unwrap();
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.writer.write_all(buf)?;
        self.hasher.write(&buf).unwrap();
        Ok(())
    }
}
