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

#[cfg(test)]
mod tests {
    use ::{ChunkInput, read_chunks};

    use std::io::Cursor;

    #[test]
    fn test() {
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
}
