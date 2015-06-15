use bencode::BItem;


#[derive(Debug, PartialEq, Eq)]
pub enum ID {
    Bytes([u8; 20]),
}


fn parse_hex(c: u8) -> Option<u8> {
    if b'0' <= c && c <= b'9' {
        Some(c - b'0')
    } else if b'a' <= c && c <= b'f' {
        Some(c - b'a' + 10)
    } else if b'A' <= c && c <= b'F' {
        Some(c - b'A' + 10)
    } else {
        None
    }
}

impl ID {
    pub fn new(bytes: &[u8]) -> Option<ID> {
        if bytes.len() == 20 {
            let mut new = [0u8; 20];
            for (i, b) in bytes.iter().enumerate() {
                new[i] = b.clone();
            }
            Some(ID::Bytes(new))
        } else {
            None
        }
    }

    pub fn parse(text: &[u8]) -> Option<ID> {
        if text.len() == 40 {
            let mut new = [0u8; 20];
            for i in 0..20 {
                if let Some(a) = parse_hex(text[i * 2]) {
                    if let Some(b) = parse_hex(text[i * 2 + 1]) {
                        new[i] = a << 4 | b;
                    } else {
                        return None
                    }
                } else {
                    return None
                }
            }
            Some(ID::Bytes(new))
        } else {
            None
        }
    }
}

#[test]
fn test_parse() {
    assert_eq!(
        ID::parse(b"aaf4c61ddcc5e8a2dabeDE0F3B482CD9AEA9434D"),
        Some(ID::Bytes([
            0xAA, 0xF4, 0xC6, 0x1D, 0xDC, 0xC5, 0xE8, 0xA2, 0xDA, 0xBE,
            0xDE, 0x0F, 0x3B, 0x48, 0x2C, 0xD9, 0xAE, 0xA9, 0x43, 0x4D])));
    assert_eq!(ID::parse(b"aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434"),
               None);
    assert_eq!(ID::parse(b"aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d3"),
               None);
    assert_eq!(ID::parse(b"aaf=c61ddcc5e8a2dabede0f3b482cd9aea9434d"),
               None);
}

impl From<[u8; 20]> for ID {
    fn from(bytes: [u8; 20]) -> ID {
        ID::Bytes(bytes)
    }
}


/// An archive.
///
/// This is a namespace containing multiple objects. It can be looked up on the
/// DHT.
///
/// It is associated with access permission and contains indexed objects.
// TODO: use Object here?
struct Archive {
    id: ID,
}


/// An object, i.e. content-addressable, bencoded, indexable data.
pub struct Object {
    id: ID,
    data: BItem,
}


/// A blob, i.e. content-addressable, raw binary data.
pub struct Blob {
    id: ID,
    data: Vec<u8>,
}


/// Trait for content-addressable data.
///
/// Uses SHA1.
pub trait ContentAddressable {
    fn id(&self) -> &ID;
}

impl ContentAddressable for Object {
    fn id(&self) -> &ID {
        return &self.id;
    }
}

impl ContentAddressable for Blob {
    fn id(&self) -> &ID {
        return &self.id;
    }
}
