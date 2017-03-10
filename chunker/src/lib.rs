use std::io::{self, Read};
use std::num::Wrapping;

pub enum ChunkInput<'a> {
    Data(&'a [u8]),
    End,
}

#[cfg(not(test))]
const BUF_SIZE: usize = 4096;
#[cfg(test)]
const BUF_SIZE: usize = 8;

const HM: Wrapping<u32> = Wrapping(123456791);

pub fn read_chunks<R: Read, F: FnMut(ChunkInput)>
    (mut reader: R, nbits: usize, mut callback: F) -> io::Result<()>
{
    let nbits = 32 - nbits;
    let mut c1 = 0u8; // previous byte
    let mut o1 = [0u8; 256];
    let mut h = HM;
    let mut chunk_emitted = false;

    loop {
        let mut buf = [0u8; BUF_SIZE];
        let len = reader.read(&mut buf)?;
        if len == 0 {
            if chunk_emitted {
                callback(ChunkInput::End);
            }
            return Ok(());
        } else {
            let mut s = 0;
            for (i, &c) in buf[..len].iter().enumerate() {
                if c == o1[c1 as usize] {
                    h = h * HM + Wrapping(c as u32 + 1);
                } else {
                    h = h * HM * Wrapping(2) + Wrapping(c as u32 + 1);
                }
                o1[c1 as usize] = c;
                c1 = c;

                if h.0 < (1 << nbits) {
                    callback(ChunkInput::Data(&buf[s..(i + 1)]));
                    callback(ChunkInput::End);
                    chunk_emitted = false;
                    s = i + 1;
                    c1 = 0u8;
                    o1 = [0u8; 256];
                    h = HM;
                }
            }
            if s < len {
                callback(ChunkInput::Data(&buf[s..len]));
                chunk_emitted = true;
            }
        }
    }
}

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
    // Can't be iterator because of 'a
    pub fn next_chunk<'a>(&'a mut self) -> Option<io::Result<ChunkInput<'a>>> {
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
    use ::{ChunkInput, read_chunks, chunks};

    use std::io::Cursor;

    #[test]
    fn test_callback() {
        let input = "abcdefghijklmnopqrstuvwxyz1234567890";
        let mut out = Vec::new();
        read_chunks(Cursor::new(input), 3, |a| match a {
            ChunkInput::Data(d) => {
                out.extend_from_slice(d);
                out.push(b'.');
            }
            ChunkInput::End => out.push(b'|'),
        }).unwrap();

        let expected: &[u8] =
            b"abcdefgh.ijk.|lmno.|p.q.|rstuvw.|x.yz123.|456.7890.|";
        assert_eq!(out, expected);
    }

    #[test]
    fn test_iter() {
        let input = "abcdefghijklmnopqrstuvwxyz1234567890";
        let mut out = Vec::new();
        let mut chunk_iter = chunks(Cursor::new(input), 3);
        while let Some(chunk) = chunk_iter.next_chunk() {
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
