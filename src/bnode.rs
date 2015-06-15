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
        }
    }
}


/// An item in a bencoded message.
///
/// This is either an integer, a bytestring, a list or a dictionary (with
/// bytestring keys).
#[derive(PartialEq, Eq)]
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
                for e in v.iter() {
                    try!(write!(f, "{:?}", e));
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


impl BItem {
    /// Parse a dhstore BNode as a tree of BItems.
    ///
    /// Note that BNodes are required to have their keys ordered in
    /// dictionaries; use parse_raw() if parsing DHT messages where this
    /// behavior is not expected.
    pub fn parse(bencoded: &[u8]) -> Result<BItem, BDecodeError> {
        Self::parse_internal(bencoded, false, 0)
    }

    /// Parse a bencoded message.
    ///
    /// Note that this doesn't check the ordering of keys in dictionaries
    /// (BDecodeError::OutOfOrderKey will never be returned); use parse() if
    /// parsing a dhstore BNode where this behavior is expected.
    pub fn parse_raw(bencoded: &[u8]) -> Result<BItem, BDecodeError> {
        Self::parse_internal(bencoded, true, 0)
    }

    fn parse_internal(bencoded: &[u8], allow_out_of_order: bool, depth: u32)
            -> Result<BItem, BDecodeError> {
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
                if bencoded[pos] == b'0' && val == 0 && pos > 1 {
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
            } else if pos != bencoded.len() -1 || pos < 2 ||
                    bencoded[pos] != b'e' {
                Err(BDecodeError::ParseError)
            } else {
                Ok(BItem::Integer(sign * val))
            }
        // List
        } else if bencoded[0] == b'l' {
            panic!();
        // Dictionary
        } else if bencoded[0] == b'd' {
            panic!();
        // Bytestring
        } else if is_digit(bencoded[0]) {
            panic!();
        } else {
            Err(BDecodeError::ParseError)
        }
    }
}

#[test]
fn test_bdecode() {
    assert_eq!(BItem::parse_raw("i12e".as_bytes()),
               Ok(BItem::Integer(12)));
    assert_eq!(BItem::parse_raw("i0e".as_bytes()),
               Ok(BItem::Integer(0)));
    assert_eq!(BItem::parse_raw("i00e".as_bytes()),
               Err(BDecodeError::ParseError));
    assert_eq!(BItem::parse_raw("i-4e".as_bytes()),
               Ok(BItem::Integer(-4)));
    assert_eq!(BItem::parse_raw("ie".as_bytes()),
               Err(BDecodeError::ParseError));
    assert_eq!(BItem::parse_raw("i".as_bytes()),
               Err(BDecodeError::UnexpectedEOF));
    assert_eq!(BItem::parse_raw("i123".as_bytes()),
               Err(BDecodeError::UnexpectedEOF));
}
