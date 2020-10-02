//! Structures and functions related to hashing.
//!
//! This module contains the `ID` type used to addres blobs and objects by their
//! content, as well as `Hasher` used to build it from bytes.

use sha2::{Digest, Sha256};
use std::fmt;
use std::io::{self, Read, Write};
use std::hash;

/// Identifier for an object.
///
/// Because they are content-addressable, this is a hash of its content.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ID {
    pub bytes: [u8; 32],
}

const BASE64_CHARS: &'static [u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const BASE64_BYTES: &'static [u8] = &[
    64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64,
    64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64,
    64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 62, 64, 64,
    52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 64, 64, 64, 64, 64, 64,
    64, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
    15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 64, 64, 64, 64, 63,
    64, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
    41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 64, 64, 64, 64, 64,
];

/// Size of the hash in bytes.
///
/// This module uses SHA256, therefore this is 32.
pub const HASH_SIZE: usize = 32;

impl ID {
    /// Make an ID from raw bytes.
    pub fn from_bytes(buf: &[u8]) -> Option<ID> {
        if buf.len() == 32 {
            let mut bytes = [0u8; 32];
            bytes.clone_from_slice(buf);
            Some(ID { bytes: bytes })
        } else {
            None
        }
    }

    /// Returns a string representation of the ID.
    ///
    /// This is the hash in base64.
    pub fn str(&self) -> String {
        fn b64(byte: u8) -> u8 {
            BASE64_CHARS[63 & (byte as usize)]
        }

        let mut hashstr = vec![0u8; 44];
        let bytes = &self.bytes;
        let code = 12u8;
        hashstr[0] = b64(code >> 2                                );
        hashstr[1] = b64(code << 4 | bytes[0] >> 4                );
        hashstr[2] = b64(            bytes[0] << 2 | bytes[1] >> 6);
        hashstr[3] = b64(                            bytes[1]     );
        for i in 0..10 {
            let c = 4 * (i + 1);
            let b = &self.bytes[2 + i * 3..];
            hashstr[c    ] = b64(b[0] >> 2                        );
            hashstr[c + 1] = b64(b[0] << 4 | b[1] >> 4            );
            hashstr[c + 2] = b64(            b[1] << 2 | b[2] >> 6);
            hashstr[c + 3] = b64(                        b[2]     );
        }
        unsafe { String::from_utf8_unchecked(hashstr) }
    }

    /// Parses the string representation into a ID.
    ///
    /// This returns an `ID` if the string was valid, else None.
    pub fn from_str(hashstr: &[u8]) -> Option<ID> {
        macro_rules! b64 {
            ( $chr:expr ) => {
                {
                    let _chr = $chr;
                    if _chr < 128u8 {
                        let _byte = BASE64_BYTES[_chr as usize];
                        if _byte == 64 {
                            return None;
                        } else {
                            _byte
                        }
                    } else {
                        return None
                    }
                }
            };
        }

        if hashstr.len() != 44 {
            return None;
        }
        let code = b64!(hashstr[0]) << 2 | b64!(hashstr[1]) >> 4;
        if code != 12 {
            return None;
        }
        let mut out = [0u8; 32];
        out[0] = b64!(hashstr[1]) << 4 | b64!(hashstr[2]) >> 2;
        out[1] = b64!(hashstr[2]) << 6 | b64!(hashstr[3]);
        for i in 0..10 {
            let b = 2 + i * 3;
            let s = &hashstr[4 * (i + 1)..];
            out[b    ] = b64!(s[0]) << 2 | b64!(s[1]) >> 4;
            out[b + 1] = b64!(s[1]) << 4 | b64!(s[2]) >> 2;
            out[b + 2] = b64!(s[2]) << 6 | b64!(s[3]);
        }
        Some(ID { bytes: out })
    }
}

impl hash::Hash for ID {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

impl fmt::Display for ID {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.str())?;
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
#[derive(Default)]
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
    writer: W,
    hasher: Hasher,
}

impl<W: Write> HasherWriter<W> {
    /// Build a new `HasherWriter` that will write on the given object.
    pub fn new(writer: W) -> HasherWriter<W> {
        HasherWriter::with_hasher(writer, Hasher::new())
    }

    pub fn with_hasher(writer: W, hasher: Hasher) -> HasherWriter<W> {
        HasherWriter {
            writer: writer,
            hasher: hasher,
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
        self.hasher.write_all(&buf[..len]).unwrap();
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.writer.write_all(buf)?;
        self.hasher.write_all(buf).unwrap();
        Ok(())
    }
}

/// A convenient adapter to hash while reading.
///
/// Read operations on this object will be forwarded to the given object,
/// while computing a hash.
pub struct HasherReader<R: Read> {
    reader: R,
    hasher: Hasher,
}

impl<R: Read> HasherReader<R> {
    /// Build a new `HasherReader` that will read from the given object.
    pub fn new(reader: R) -> HasherReader<R> {
        HasherReader::with_hasher(reader, Hasher::new())
    }

    pub fn with_hasher(reader: R, hasher: Hasher) -> HasherReader<R> {
        HasherReader {
            reader: reader,
            hasher: hasher,
        }
    }

    /// Consume this object and returns the `ID` computed from hashing.
    ///
    /// The internal `Read` object given at construction is dropped.
    pub fn result(self) -> ID {
        self.hasher.result()
    }
}

impl<R: Read> Read for HasherReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = self.reader.read(buf)?;
        self.hasher.write_all(&buf[..len]).unwrap();
        Ok(len)
    }
}

#[cfg(test)]
mod tests {
    use super::ID;

    fn run_tests(check: &Fn(&[u8], &str)) {
        check(b"abcdefghijklmnopqrstuvwxyz123456",
              "DGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6MTIzNDU2");
        check(b"\x00bcd\xF4fg\x7Fijkl\x88nop\x00rstuvwxyz123\xC9\xFF\xDE",
              "DABiY2T0Zmd_aWprbIhub3AAcnN0dXZ3eHl6MTIzyf_e");
        check(b"\xFFbcd\xF4fg\x7Fijkl\x88nop\x00rstuvwxyz123\x14\x03\x00",
              "DP9iY2T0Zmd_aWprbIhub3AAcnN0dXZ3eHl6MTIzFAMA");
    }

    #[test]
    fn test_encode() {
        fn check(bin: &[u8], enc: &str) {
            assert_eq!(ID::from_bytes(bin).unwrap().str(),
                       enc);
        }
        run_tests(&check);
    }

    #[test]
    fn test_decode() {
        fn check(bin: &[u8], enc: &str) {
            assert_eq!(ID::from_str(enc.as_bytes()).unwrap().bytes,
                       bin);
        }
        run_tests(&check);
    }
}
