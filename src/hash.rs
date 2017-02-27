use sha1::Sha1;

/// Identifier for an object.
///
/// Because they are content-addressable, this is a hash of its content.
pub struct ID {
    pub bytes: [u8; 20],
}

impl ID {
    pub fn from_slice(buf: &[u8]) -> Option<ID> {
        if buf.len() == 20 {
            let mut bytes = [0u8; 20];
            bytes.clone_from_slice(buf);
            Some(ID { bytes: bytes })
        } else {
            None
        }
    }

    pub fn hash_size() -> usize {
        20
    }
}

/// Content to ID code.
///
/// Abstracted to make it easier to swap it out, or use multiple hashes,
/// but there is no current plan to make the lib generic on this.
pub struct Hasher {
    hasher: Sha1,
}

impl Hasher {
    pub fn new() -> Hasher {
        Hasher { hasher: Sha1::new() }
    }

    pub fn update(&mut self, msg: &[u8]) {
        self.hasher.update(msg);
    }

    pub fn result(self) -> ID {
        ID { bytes: self.hasher.digest().bytes() }
    }
}
