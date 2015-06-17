use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::io::Write;


/// Simple utility function to build a Vec<u8> from a b"..." literal.
///
/// Usually imported with `use bencode::vec_from_slice as v;`.
/// Currently, `let a: Vec<u8> = b"hello".to_owned();` doesn't compile. This
/// allows you to do `let a: Vec<u8> = v(b"hello");` instead.
pub fn vec_from_slice<T>(s: &[T]) -> Vec<T>
        where [T]: ToOwned<Owned=Vec<T>> {
    s.to_owned()
}

use self::vec_from_slice as v;


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

    /// Write out bencoded object as a bytestring.
    ///
    /// Dictionaries are written with their keys in order.
    pub fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();
        self.serialize_internal(&mut result);
        result
    }

    fn serialize_internal(&self, result: &mut Vec<u8>) {
        match *self {
            BItem::Integer(i) => write!(result, "i{}e", i).unwrap(),
            BItem::Bytestring(ref v) => Self::serialize_bytestring(v, result),
            BItem::List(ref v) => {
                result.push(b'l');
                for e in v {
                    e.serialize_internal(result);
                }
                result.push(b'e');
            },
            BItem::Dictionary(ref m) => {
                let mut l = m.iter().collect::<Vec<(&Vec<u8>, &BItem)>>();
                l.sort_by(|&(k1, _), &(k2, _)| k1.cmp(k2));
                result.push(b'd');
                for (k, v) in l {
                    Self::serialize_bytestring(k, result);
                    v.serialize_internal(result);
                }
                result.push(b'e');
            },
        }
    }

    fn serialize_bytestring(bytes: &[u8], result: &mut Vec<u8>) {
        write!(result, "{}:", bytes.len()).unwrap();
        result.extend(bytes.iter().cloned());
    }
}


macro_rules! assert_encodes {
    ( $enc:expr, $dec:expr ) => {
        assert_eq!(BItem::parse($enc), Ok($dec));
        assert_eq!(BItem::parse($enc).unwrap().serialize(), $enc);
    };
}

macro_rules! assert_error {
    ( $enc:expr, $err:expr ) => {
        assert_eq!(BItem::parse($enc), Err($err));
    }
}

#[test]
fn test_integer() {
    assert_encodes!(b"i12e", BItem::Integer(12));
    assert_encodes!(b"i0e", BItem::Integer(0));
    assert_encodes!(b"i10e", BItem::Integer(10));
    assert_error!(b"i01e", BDecodeError::ParseError);
    assert_encodes!(b"i-4e", BItem::Integer(-4));
    assert_error!(b"ie", BDecodeError::ParseError);
    assert_error!(b"i", BDecodeError::UnexpectedEOF);
    assert_error!(b"i123", BDecodeError::UnexpectedEOF);
}

#[test]
fn test_list() {
    assert_encodes!(b"le", BItem::List(vec![]));
    assert_encodes!(b"li12ei5ee",
                    BItem::List(vec![BItem::Integer(12), BItem::Integer(5)]));
    assert_error!(b"lle", BDecodeError::UnexpectedEOF);
    assert_encodes!(b"li-1eli2ei3eee",
                    BItem::List(vec![
                        BItem::Integer(-1),
                        BItem::List(vec![BItem::Integer(2),
                                         BItem::Integer(3)])]));
    assert_error!(&[b'l'; 128], BDecodeError::DepthExceeded);
}

#[test]
fn test_bytes() {
    assert_encodes!(b"0:", BItem::Bytestring(vec![]));
    assert_encodes!(b"1:a", BItem::Bytestring(v(b"a")));
    assert_encodes!(b"5:hello", BItem::Bytestring(v(b"hello")));
    assert_error!(b"6:hello", BDecodeError::UnexpectedEOF);
    assert_error!(b"4:hello", BDecodeError::TrailingTokens);
    assert_encodes!(b"10:helloworld", BItem::Bytestring(v(b"helloworld")));
    assert_error!(b"01:a", BDecodeError::ParseError);
}

#[test]
fn test_dictionary() {
    assert_encodes!(b"de", BItem::Dictionary(HashMap::new()));
    assert_error!(b"d5:hello", BDecodeError::UnexpectedEOF);
    assert_error!(b"di1ei2ee", BDecodeError::NonBytesKey);
    assert_error!(b"d5:helloi1e", BDecodeError::UnexpectedEOF);
    assert_encodes!(b"d5:helloi1e3:who5:worlde",
        BItem::Dictionary([
            (v(b"hello"),
             BItem::Integer(1)),
            (v(b"who"),
             BItem::Bytestring(v(b"world")))
            ].iter().cloned().collect()));
    assert!(BItem::parse_raw(b"d2:bbi4e2:aai4ee").is_ok());
    assert_error!(b"d2:bbi4e2:aai4ee", BDecodeError::OutOfOrderKey);
    assert_error!(b"d2:aai4e2:aai4ee", BDecodeError::DuplicatedKey);
}
