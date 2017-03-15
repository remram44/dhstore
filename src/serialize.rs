//! Serialization & deserialization code for DHStore objects.
//!
//! This contains the `serialize()` and `deserialize()` methods that convert
//! objects to and from bytes. The `hash_object()` function, that takes object
//! content and hash it to tack on it the `ID` that makes an `Object`, is also
//! here, since the same serialization format is used for hashing.

use std::collections::BTreeMap;
use std::io::{self, Read, Write};

use common::{ID, Dict, List, Object, ObjectData, Property};
use hash::{Hasher, HasherReader, HasherWriter};

// Dictionary: d<id><key><value><key><value>...e
// List: l<value><value>...e
// String: 5:hello
// Integer: i42e
// Reference: {"ref": d} = d3:ref64:abcdef...e
// Blob: {"blob": id} = d4:blob64:abcdef...e
// Object: {"d": "dhstore_0001", "r": ...}
//   r: either a list or a dict

macro_rules! invalid {
    () => {
        {
            error!("invalid object");
            return Err(io::ErrorKind::InvalidData.into())
        }
    };
    ( $fmt:expr ) => {
        invalid!($fmt,)
    };
    ( $fmt:expr, $( $arg:expr ),+ ) => {
        invalid!( $fmt, $( $arg, )+ )
    };
    ( $fmt:expr, $( $arg:expr, )* ) => {
        {
            error!(concat!("deserialize: ", $fmt), $( $arg, )* );
            return Err(io::ErrorKind::InvalidData.into())
        }
    };
}

fn write_ref<W: Write>(out: &mut W, id: &ID, blob: bool) -> io::Result<()> {
    out.write_all(if blob { b"d4:blob" } else { b"d3:ref" })?;
    write_str(out, &id.str())?;
    out.write_all(b"e")
}

fn write_str<W: Write>(out: &mut W, string: &str) -> io::Result<()> {
    write!(out, "{}:{}", string.len(), string)
}

fn write_property<W: Write>(out: &mut W, prop: &Property) -> io::Result<()> {
    match *prop {
        Property::String(ref s) => write_str(out, s),
        Property::Integer(i) => write!(out, "i{}e", i),
        Property::Reference(ref id) => write_ref(out, id, false),
        Property::Blob(ref id) => write_ref(out, id, true),
    }
}

fn write_data<W: Write>(out: &mut W, data: &ObjectData)
    -> io::Result<()>
{
    match *data {
        ObjectData::Dict(ref d) => {
            out.write_all(b"d")?;
            for (key, value) in d {
                write_str(out, key)?;
                write_property(out, value)?;
            }
            out.write_all(b"e")?;
        }
        ObjectData::List(ref l) => {
            out.write_all(b"l")?;
            for value in l {
                write_property(out, value)?;
            }
            out.write_all(b"e")?;
        }
    }
    Ok(())
}

/// Write out the object on the given `Write` handle.
pub fn serialize<W: Write>(mut out: &mut W, object: &Object) -> io::Result<()> {
    out.write_all(b"d\
                    1:d12:dhstore_0001\
                    1:r")?;
    if cfg!(debug_assertions) || cfg!(test) {
        let mut hasher = Hasher::new();
        hasher.write_all(b"object\n").unwrap();
        let mut hasherwriter = HasherWriter::with_hasher(&mut out, hasher);
        write_data(&mut hasherwriter, &object.data)?;
        if hasherwriter.result() != object.id {
            panic!("serializing an object yielded a different ID");
        }
    } else {
        write_data(out, &object.data)?;
    }
    out.write_all(b"e")
}

fn read_byte<R: Read>(read: &mut R) -> io::Result<u8> {
    let mut buf = [0u8; 1];
    if read.read(&mut buf)? == 0 {
        Err(io::ErrorKind::UnexpectedEof.into())
    } else {
        Ok(buf[0])
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Item {
    String(String),
    Integer(i64),
    Dict(BTreeMap<String, Item>),
    List(Vec<Item>),
    End,
}

impl Item {
    fn str(&self) -> Option<&str> {
        match *self {
            Item::String(ref s) => Some(s),
            _ => None,
        }
    }
}

fn read_item<R: Read>(read: &mut R) -> io::Result<Item> {
    match read_byte(read)? {
        b'd' => {
            let mut dict = BTreeMap::new();
            loop {
                let key = match read_item(read)? {
                    Item::End => return Ok(Item::Dict(dict)),
                    Item::String(s) => s,
                    _ => invalid!("invalid dict key"),
                };
                if let Some(last) = dict.keys().next_back() {
                    if last > &key {
                        invalid!("dict key {:?} is out of order", key);
                    }
                }
                if dict.get(&key).is_some() {
                    invalid!("duplicate key {:?} in dict", key);
                }
                let value = match read_item(read)? {
                    Item::End => invalid!("missing value for key {:?} in dict",
                                          key),
                    v => v,
                };
                dict.insert(key, value);
            }
        }
        b'l' => {
            let mut list = Vec::new();
            loop {
                match read_item(read)? {
                    Item::End => return Ok(Item::List(list)),
                    v => list.push(v),
                }
            }
        }
        c @ b'0'...b'9' => {
            let mut len = (c - b'0') as usize;
            loop {
                let c = read_byte(read)?;
                if b'0' <= c && c <= b'9' {
                    len = len * 10 + (c - b'0') as usize;
                } else if c == b':' {
                    let mut s = String::new();
                    for _ in 0..len {
                        s.push(read_byte(read)? as char);
                    }
                    return Ok(Item::String(s));
                } else {
                    invalid!("invalid string length");
                }
            }
        }
        b'i' => {
            let mut nb: i64 = 0;
            loop {
                let d = read_byte(read)?;
                if b'0' <= d && d <= b'9' {
                    let (n, o) = nb.overflowing_mul(10);
                    if o {
                        invalid!("integer overflow");
                    }
                    let (n, o) = n.overflowing_add((d - b'0') as i64);
                    if o {
                        invalid!("integer overflow");
                    }
                    nb = n;
                } else if d == b'e' {
                    return Ok(Item::Integer(nb));
                } else {
                    invalid!("invalid character in integer");
                }
            }
        }
        b'e' => Ok(Item::End),
        _ => invalid!("invalid item"),
    }
}

fn convert_property(item: Item) -> Option<Property> {
    match item {
        Item::String(s) => return Some(Property::String(s)),
        Item::Integer(i) => return Some(Property::Integer(i)),
        Item::Dict(d) => {
            if d.len() == 1 {
                let (k, v) = d.into_iter().next().unwrap();
                if let Some(v) = v.str().map(str::as_bytes)
                    .and_then(ID::from_str)
                {
                    return match &k[..] {
                        "ref" => Some(Property::Reference(v)),
                        "blob" => Some(Property::Blob(v)),
                        _ => None,
                    };
                }
            }
        }
        _ => {}
    }
    None
}

fn expect<R: Read>(mut read: R, what: &[u8]) -> io::Result<()> {
    let mut buf = [0u8; 4];
    let buf = &mut buf[0..what.len()]; // FIXME: rust-lang/rfcs#618
    read.read_exact(buf)?;
    if buf != what {
        invalid!();
    }
    Ok(())
}

/// Read an Object from the given `Read` handle.
pub fn deserialize<R: Read>(mut read: R) -> io::Result<Object> {
    expect(&mut read, b"d1:d")?;
    let obj = read_item(&mut read)?;
    match obj {
        Item::String(s) => {
            if s != "dhstore_0001" {
                invalid!("unknown format {:?}", s);
            }
        }
        _ => invalid!(),
    }
    expect(&mut read, b"1:r")?;
    let (obj, id) = {
        let mut hasher = Hasher::new();
        hasher.write_all(b"object\n").unwrap();
        let mut reader = HasherReader::with_hasher(&mut read, hasher);
        let obj = read_item(&mut reader)?;
        (obj, reader.result())
    };
    expect(&mut read, b"e")?;
    if read.read(&mut [0u8])? != 0 {
        invalid!("trailing bytes");
    }

    let data = match obj {
        Item::Dict(d) => {
            let mut dict = Dict::new();
            for (k, v) in d {
                match convert_property(v) {
                    Some(v) => { dict.insert(k, v); }
                    None => invalid!("invalid dict value"),
                }
            }
            ObjectData::Dict(dict)
        }
        Item::List(l) => {
            let mut list = List::new();
            for v in l {
                match convert_property(v) {
                    Some(v) => list.push(v),
                    None => invalid!("invalid list value"),
                }
            }
            ObjectData::List(list)
        }
        _ => invalid!("invalid object type"),
    };
    let object = Object {
        id: id,
        data: data,
    };
    Ok(object)
}

/// Hash the given object data, and tack on the digest to form an `Object`.
pub fn hash_object(data: ObjectData) -> Object {
    let mut hasher = Hasher::new();
    hasher.write_all(b"object\n").unwrap();
    write_data(&mut hasher, &data).unwrap();
    Object {
        id: hasher.result(),
        data: data,
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use common::{ID, Dict, List, ObjectData, Property};
    use serialize::{hash_object, serialize, deserialize};

    fn fake_id(digit: u8) -> ID {
        let mut s = [b'0' + digit as u8; 44];
        s[0] = b'D';
        s[1] = b'B';
        ID::from_str(&s).unwrap()
    }

    const TEST_DICT: &'static [u8] =
        b"d\
          1:d12:dhstore_0001\
          1:rd\
          6:camera\
          d3:ref44:DB11111111111111111111\
          1111111111111111111111e\
          4:data\
          d4:blob44:DB22222222222222222222\
          2222222222222222222222e\
          8:filename\
          22:DSC_20170303223104.jpg\
          6:people\
          i5e\
          ee";

    #[test]
    fn test_serialize_dict() {
        // Create properties
        let mut properties = Dict::new();
        properties.insert("filename".into(),
                          Property::String("DSC_20170303223104.jpg".into()));
        properties.insert("people".into(), Property::Integer(5));
        properties.insert("camera".into(), Property::Reference(fake_id(1)));
        properties.insert("data".into(), Property::Blob(fake_id(2)));
        let hash = ID::from_str(b"DNbf17WpaH2XJC5tRWYhMO\
                                  TQdMt2TSutfKAp3wnKoIV7").unwrap();
        let obj = hash_object(ObjectData::Dict(properties));
        assert_eq!(obj.id, hash);
        let mut serialized = Vec::new();
        serialize(&mut serialized, &obj).unwrap();
        assert_eq!(serialized, TEST_DICT);
    }

    #[test]
    fn test_deserialize_dict() {
        let obj = deserialize(Cursor::new(TEST_DICT)).unwrap();
        assert_eq!(obj.id,
                   ID::from_str(b"DNbf17WpaH2XJC5tRWYhMO\
                                  TQdMt2TSutfKAp3wnKoIV7").unwrap());
    }

    const TEST_LIST: &'static [u8] =
        b"d\
          1:d12:dhstore_0001\
          1:rl\
          3:cvs\
          10:subversion\
          5:darcs\
          3:git\
          9:mercurial\
          ee";

    #[test]
    fn test_serialize_list() {
        // Create properties
        let properties: List = ["cvs", "subversion", "darcs", "git", "mercurial"]
            .iter()
            .map(|&s: &&str| -> String { s.into() })
            .map(Property::String)
            .collect();
        let hash = ID::from_str(b"DOdY4OwCEf6AouK4eK6fRs\
                                  mG6JiGoKjfe-fOJ-I29H1D").unwrap();
        let obj = hash_object(ObjectData::List(properties));
        assert_eq!(obj.id, hash);
        let mut serialized = Vec::new();
        serialize(&mut serialized, &obj).unwrap();
        assert_eq!(serialized, TEST_LIST);
    }

    #[test]
    fn test_deserialize_list() {
        let obj = deserialize(Cursor::new(TEST_LIST)).unwrap();
        assert_eq!(obj.id,
                   ID::from_str(b"DOdY4OwCEf6AouK4eK6fRs\
                                  mG6JiGoKjfe-fOJ-I29H1D").unwrap());
    }
}
