use std::collections::HashMap;
use std::error::Error;
use std::fmt;


/// Error decoding bencoded messages.
///
/// Note that dhstore's BNodes must have ordered keys, but this is not a
/// requirement of the usual bencoding scheme. Therefore, OutOfOrderKey will
/// not be returned by BItem::parse_raw(), only BItem::parse() (this is the
/// only difference).
#[derive(Debug, PartialEq, Eq)]
pub enum BDecodeError {
    ParseError,
    DuplicatedKey,
    OutOfOrderKey,
    NonBytesKey,
    TrailingTokens,
    UnexpectedEOF,
    DepthExceeded,
}

impl fmt::Display for BDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Error for BDecodeError {
    fn description(&self) -> &str {
        match *self {
            BDecodeError::ParseError => "Parse error",
            BDecodeError::DuplicatedKey => "Duplicate keys in dictionary",
            BDecodeError::OutOfOrderKey => "Out of order keys in dictionary",
            BDecodeError::NonBytesKey => "Dictionary key is not a bytestring",
            BDecodeError::TrailingTokens => "Trailing characters after root \
                                             object",
            BDecodeError::UnexpectedEOF => "Premature end of message",
            BDecodeError::DepthExceeded => "Maximum depth exceeded",
        }
    }
}


/// An item in a bencoded message.
///
/// This is either an integer, a bytestring, a list or a dictionary (with
/// bytestring keys).
#[derive(Clone, PartialEq, Eq)]
pub enum BItem {
    Integer(i32),
    Bytestring(Vec<u8>),
    List(Vec<BItem>),
    Dictionary(HashMap<Vec<u8>, BItem>),
}

impl fmt::Debug for BItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BItem::Integer(i) => write!(f, "{:?}", i),
            BItem::Bytestring(ref v) => write!(f, "{:?}", v),
            BItem::List(ref v) => {
                try!(write!(f, "["));
                for (i, e) in v.iter().enumerate() {
                    try!(write!(f, "{}{:?}",
                                if i == 0 { "" } else { ", " },
                                e));
                }
                try!(write!(f, "]"));
                Ok(())
            },
            BItem::Dictionary(ref m) => {
                try!(write!(f, "{{"));
                for (i, (k, v)) in m.iter().enumerate() {
                    try!(write!(f, "{}{:?}: {:?}",
                                if i == 0 { "" } else { ", " },
                                k, v,));
                }
                try!(write!(f, "}}"));
                Ok(())
            },
        }
    }
}


fn is_digit(b: u8) -> bool {
    b'0' <= b && b <= b'9'
}


const MAX_DEPTH: u32 = 32;

impl BItem {
    /// Parse a dhstore BNode as a tree of BItems.
    ///
    /// Note that BNodes are required to have their keys ordered in
    /// dictionaries; use parse_raw() if parsing DHT messages where this
    /// behavior is not expected.
    pub fn parse(bencoded: &[u8]) -> Result<BItem, BDecodeError> {
        Self::parse_internal(bencoded, false)
    }

    /// Parse a bencoded message.
    ///
    /// Note that this doesn't check the ordering of keys in dictionaries
    /// (BDecodeError::OutOfOrderKey will never be returned); use parse() if
    /// parsing a dhstore BNode where this behavior is expected.
    pub fn parse_raw(bencoded: &[u8]) -> Result<BItem, BDecodeError> {
        Self::parse_internal(bencoded, true)
    }

    fn parse_internal(bencoded: &[u8], allow_out_of_order: bool)
            -> Result<BItem, BDecodeError> {
        let (result, pos) = try!(
            Self::parse_internal_r(bencoded, allow_out_of_order, 0));
        if pos == bencoded.len() {
            Ok(result)
        } else {
            Err(BDecodeError::TrailingTokens)
        }
    }

    fn parse_internal_r(bencoded: &[u8], allow_out_of_order: bool, depth: u32)
            -> Result<(BItem, usize), BDecodeError> {
        if bencoded.len() < 2 {
            Err(BDecodeError::UnexpectedEOF)
        // Integer
        } else if bencoded[0] == b'i' {
            let mut pos = 1;
            let mut val: i32 = 0;
            let sign = if bencoded[1] == b'-' {
                pos += 1;
                -1
            } else {
                1
            };
            while pos < bencoded.len() && is_digit(bencoded[pos]) {
                if val == 0 && bencoded[pos - 1] == b'0' {
                    return Err(BDecodeError::ParseError);
                }
                val = {
                    let d = (bencoded[pos] - b'0') as i32;
                    // Use checked overflow operations
                    let v = val.checked_mul(10).and_then(|v| v.checked_add(d));
                    try!(v.ok_or(BDecodeError::ParseError))
                };
                pos += 1;
            }
            if pos >= bencoded.len() {
                Err(BDecodeError::UnexpectedEOF)
            } else if pos < 2 || bencoded[pos] != b'e' {
                Err(BDecodeError::ParseError)
            } else {
                Ok((BItem::Integer(sign * val), pos + 1))
            }
        // List
        } else if bencoded[0] == b'l' {
            if depth >= MAX_DEPTH {
                return Err(BDecodeError::DepthExceeded);
            }
            let mut pos = 1;
            let mut val = Vec::new();
            while pos < bencoded.len() {
                if bencoded[pos] == b'e' {
                    return Ok((BItem::List(val), pos + 1));
                }
                let (result, p) = try!(BItem::parse_internal_r(
                    &bencoded[pos..],
                    allow_out_of_order,
                    depth + 1));
                val.push(result);
                pos += p;
            }
            Err(BDecodeError::UnexpectedEOF)
        // Dictionary
        } else if bencoded[0] == b'd' {
            if depth >= MAX_DEPTH {
                return Err(BDecodeError::DepthExceeded);
            }
            let mut pos = 1;
            let mut val = HashMap::new();
            let mut last_key = None;
            while pos < bencoded.len() {
                if bencoded[pos] == b'e' {
                    return Ok((BItem::Dictionary(val), pos + 1));
                }
                let (key_item, p) = try!(BItem::parse_internal_r(
                    &bencoded[pos..],
                    allow_out_of_order,
                    depth + 1));
                pos += p;
                let key = match key_item {
                    BItem::Bytestring(bytes) => bytes,
                    _ => return Err(BDecodeError::NonBytesKey),
                };
                if let Some(ref oldkey) = last_key {
                    if oldkey == &key {
                        return Err(BDecodeError::DuplicatedKey);
                    } else if !allow_out_of_order && oldkey > &key {
                        return Err(BDecodeError::OutOfOrderKey);
                    }
                }
                last_key = Some(key.clone());
                let (value, p) = try!(BItem::parse_internal_r(
                    &bencoded[pos..],
                    allow_out_of_order,
                    depth + 1));
                pos += p;
                if val.insert(key, value).is_some() {
                    return Err(BDecodeError::DuplicatedKey);
                }
            }
            Err(BDecodeError::UnexpectedEOF)
        // Bytestring
        } else if is_digit(bencoded[0]) {
            let mut length = (bencoded[0] - b'0') as usize;
            let mut pos = 1;
            while pos < bencoded.len() && is_digit(bencoded[pos]) {
                if length == 0 && bencoded[pos - 1] == b'0' {
                    return Err(BDecodeError::ParseError);
                }
                length = {
                    let d = (bencoded[pos] - b'0') as usize;
                    // Use checked overflow operations
                    let l = length.checked_mul(10)
                        .and_then(|l| l.checked_add(d));
                    try!(l.ok_or(BDecodeError::ParseError))
                };
                pos += 1;
            }
            if pos >= bencoded.len() || bencoded[pos] != b':' {
                return Err(BDecodeError::ParseError);
            }
            pos += 1;
            if pos + length > bencoded.len() {
                return Err(BDecodeError::UnexpectedEOF);
            }
            Ok((BItem::Bytestring(bencoded[pos..pos + length].to_owned()),
                pos + length))
        } else {
            Err(BDecodeError::ParseError)
        }
    }
}

#[test]
fn test_integer() {
    assert_eq!(BItem::parse(b"i12e"),
               Ok(BItem::Integer(12)));
    assert_eq!(BItem::parse(b"i0e"),
               Ok(BItem::Integer(0)));
    assert_eq!(BItem::parse(b"i10e"),
               Ok(BItem::Integer(10)));
    assert_eq!(BItem::parse(b"i01e"),
               Err(BDecodeError::ParseError));
    assert_eq!(BItem::parse(b"i-4e"),
               Ok(BItem::Integer(-4)));
    assert_eq!(BItem::parse(b"ie"),
               Err(BDecodeError::ParseError));
    assert_eq!(BItem::parse(b"i"),
               Err(BDecodeError::UnexpectedEOF));
    assert_eq!(BItem::parse(b"i123"),
               Err(BDecodeError::UnexpectedEOF));
}

#[test]
fn test_list() {
    assert_eq!(BItem::parse(b"le"),
               Ok(BItem::List(vec![])));
    assert_eq!(BItem::parse(b"li12ei5ee"),
               Ok(BItem::List(vec![BItem::Integer(12), BItem::Integer(5)])));
    assert_eq!(BItem::parse(b"lle"),
               Err(BDecodeError::UnexpectedEOF));
    assert_eq!(BItem::parse(b"li-1eli2ei3eee"),
               Ok(BItem::List(vec![
                   BItem::Integer(-1),
                   BItem::List(vec![BItem::Integer(2), BItem::Integer(3)])])));
    assert_eq!(BItem::parse(&[b'l'; 128]),
               Err(BDecodeError::DepthExceeded));
}

#[test]
fn test_bytes() {
    assert_eq!(BItem::parse(b"0:"),
               Ok(BItem::Bytestring(vec![])));
    assert_eq!(BItem::parse(b"1:a"),
               Ok(BItem::Bytestring((b"a" as &[u8]).to_owned())));
    assert_eq!(BItem::parse(b"5:hello"),
               Ok(BItem::Bytestring((b"hello" as &[u8]).to_owned())));
    assert_eq!(BItem::parse(b"6:hello"),
               Err(BDecodeError::UnexpectedEOF));
    assert_eq!(BItem::parse(b"4:hello"),
               Err(BDecodeError::TrailingTokens));
    assert_eq!(BItem::parse(b"10:helloworld"),
               Ok(BItem::Bytestring((b"helloworld" as &[u8]).to_owned())));
    assert_eq!(BItem::parse(b"01:a"),
               Err(BDecodeError::ParseError));
}

#[test]
fn test_dictionary() {
    assert_eq!(BItem::parse(b"de"),
               Ok(BItem::Dictionary(HashMap::new())));
    assert_eq!(BItem::parse(b"d5:hello"),
               Err(BDecodeError::UnexpectedEOF));
    assert_eq!(BItem::parse(b"di1ei2ee"),
               Err(BDecodeError::NonBytesKey));
    assert_eq!(BItem::parse(b"d5:helloi1e"),
               Err(BDecodeError::UnexpectedEOF));
    assert_eq!(
        BItem::parse(b"d5:helloi1e3:who5:worlde"),
        Ok(BItem::Dictionary([
            ((b"hello" as &[u8]).to_owned(),
             BItem::Integer(1)),
            ((b"who" as &[u8]).to_owned(),
             BItem::Bytestring((b"world" as &[u8]).to_owned()))
            ].iter().cloned().collect())));
    assert!(BItem::parse_raw(b"d2:bbi4e2:aai4ee").is_ok());
    assert_eq!(BItem::parse(b"d2:bbi4e2:aai4ee"),
               Err(BDecodeError::OutOfOrderKey));
    assert_eq!(BItem::parse(b"d2:aai4e2:aai4ee"),
               Err(BDecodeError::DuplicatedKey));
}
