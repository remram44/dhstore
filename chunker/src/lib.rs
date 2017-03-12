use std::io::{self, Read};
use std::num::Wrapping;

/// Items returned by the `Chunks` iterator.
///
/// Contains either some data that is part of the current chunk, or `End`,
/// indicating the boundary between chunks.
///
/// `End` is always returned at the end of the last chunk.
pub enum ChunkInput<'a> {
    Data(&'a [u8]),
    End,
}

#[cfg(not(test))]
const BUF_SIZE: usize = 4096;
#[cfg(test)]
const BUF_SIZE: usize = 8;

const HM: Wrapping<u32> = Wrapping(123456791);

/// Iterator returned by `chunks()`.
pub struct Chunks<R: Read> {
    reader: R,
    nbits: usize,
    buffer: [u8; BUF_SIZE],
    pos: usize,
    len: usize,
    c1: u8, // previous byte
    o1: [u8; 256],
    h: Wrapping<u32>,
    chunk_emitted: bool,
}

/// Read through a file, splitting it into chunk defined by a rolling hash.
///
/// This makes an iterator that will emit the bytes from a file, indicating when
/// a chunk boundary has been found.
///
/// The iterator's items are `ChunkInput` objects, that are either some data, or
/// a chunk boundary. Therefore chunks are not returned in one go but streamed
/// out for speed.
///
/// `nbits` indicates how many leftmost bits have to be zeros in the 32-bit
/// rolling hash value for it to be a boundary; thus `8` makes chunks of 256
/// bytes in average, `16` chunks of 64 KiB in average, ...
pub fn chunks<R: Read>(reader: R, nbits: usize) -> Chunks<R> {
    Chunks {
        reader: reader,
        nbits: 32 - nbits,
        buffer: [0u8; BUF_SIZE],
        pos: 0,
        len: 0,
        c1: 0,
        o1: [0; 256],
        h: HM,
        chunk_emitted: false,
    }
}

impl<R: Read> Chunks<R> {
    /// Iterate on the chunks, returning `ChunkInput` items.
    ///
    /// An item is either some data that is part of the current chunk, or `End`,
    /// indicating the boundary between chunks.
    ///
    /// `End` is always returned at the end of the last chunk.
    // Can't be iterator because of 'a
    pub fn read<'a>(&'a mut self) -> Option<io::Result<ChunkInput<'a>>> {
        if self.pos == self.len {
            self.pos = 0;
            self.len = match self.reader.read(&mut self.buffer) {
                Ok(l) => l,
                Err(e) => return Some(Err(e)),
            };
            if self.len == 0 {
                if self.chunk_emitted {
                    self.chunk_emitted = false;
                    return Some(Ok(ChunkInput::End));
                }
                return None;
            }
        }
        if self.h.0 < (1 << self.nbits) && self.chunk_emitted {
            self.chunk_emitted = false;
            self.c1 = 0u8;
            self.o1.clone_from_slice(&[0u8; 256]);
            self.h = HM;
            return Some(Ok(ChunkInput::End));
        }
        let mut pos = self.pos;
        while pos < self.len {
            let c = self.buffer[pos];
            if c == self.o1[self.c1 as usize] {
                self.h = self.h * HM + Wrapping(c as u32 + 1);
            } else {
                self.h = self.h * HM * Wrapping(2) + Wrapping(c as u32 + 1);
            }
            self.o1[self.c1 as usize] = c;
            self.c1 = c;

            if self.h.0 < (1 << self.nbits) {
                self.chunk_emitted = true;
                let start = self.pos;
                self.pos = pos + 1;
                return Some(Ok(
                    ChunkInput::Data(&self.buffer[start..self.pos])));
            }

            pos += 1;
        }
        let start = self.pos;
        self.pos = pos;
        self.chunk_emitted = true;
        Some(Ok(ChunkInput::Data(&self.buffer[start..self.len])))
    }
}

#[cfg(test)]
mod tests {
    use ::{ChunkInput, chunks};

    use std::io::Cursor;

    #[test]
    fn test_iter() {
        let input = "abcdefghijklmnopqrstuvwxyz1234567890";
        let mut out = Vec::new();
        let mut chunk_iter = chunks(Cursor::new(input), 3);
        while let Some(chunk) = chunk_iter.read() {
            let chunk = chunk.unwrap();
            match chunk {
                ChunkInput::Data(d) => {
                    out.extend_from_slice(d);
                    out.push(b'.');
                }
                ChunkInput::End => out.push(b'|'),
            }
        }

        println!("{}", unsafe { ::std::str::from_utf8_unchecked(&out) });
        let expected: &[u8] =
            b"abcdefgh.ijk.|lmno.|p.q.|rstuvw.|x.yz123.|456.7890.|";
        assert_eq!(out, expected);
    }
}
