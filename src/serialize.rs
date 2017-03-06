use std::collections::BTreeMap;
use std::io::{self, Read, Write};

use common::{ID, Object, ObjectData, Property};
use hash::Hasher;

// Dictionary: d<id><key><value><key><value>...e
// List: l<value><value>...e
// String: 5:hello
// Integer: i42e
// Reference: {"ref": d} = d3:ref64:abcdef...e
// Blob: {"blob": id} = d4:blob64:abcdef...e
// Object: {"d": "dhstore_0001", "h": id, "i": "l/o/p/c", "r": ...}
//   "i":
//     "o": object, "r": {...}
//     "l": list, "r": [...]
//     "p": permanode, "r": {...}
//     "c": claim, "r": {...}

macro_rules! invalid {
    () => {
        {
            error!("invalid! with no message");
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
    write_str(out, &id.hex())?;
    out.write_all(b"e")
}

fn write_str<W: Write>(out: &mut W, string: &str) -> io::Result<()> {
    write!(out, "{}:{}", string.len(), string)
}

fn write_property<W: Write>(out: &mut W, prop: &Property) -> io::Result<()> {
    match prop {
        &Property::String(ref s) => write_str(out, s),
        &Property::Integer(i) => write!(out, "i{}e", i),
        &Property::Reference(ref id) => write_ref(out, id, false),
        &Property::Blob(ref id) => write_ref(out, id, true),
    }
}

fn write_data<W: Write>(out: &mut W, data: &ObjectData)
    -> io::Result<()>
{
    match data {
        &ObjectData::Dict(ref d) => {
            out.write_all(b"1:i1:o1:rd")?;
            for (key, value) in d {
                write_str(out, key)?;
                write_property(out, value)?;
            }
            out.write_all(b"e")?;
        }
        &ObjectData::List(ref l) => {
            out.write_all(b"1:i1:l1:rl")?;
            for value in l {
                write_property(out, value)?;
            }
            out.write_all(b"e")?;
        }
    }
    Ok(())
}

pub fn serialize<W: Write>(out: &mut W, object: &Object) -> io::Result<()> {
    out.write_all(b"d\
                    1:d12:dhstore_0001\
                    1:h")?;
    write_str(out, &object.id.hex())?;
    write_data(out, &object.data)?;
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
                let value = match read_item(read)? {
                    Item::End => invalid!("missing value for key {:?} in dict",
                                          key),
                    v => v,
                };
                if dict.get(&key).is_some() {
                    invalid!("duplicate key {:?} in dict", key);
                }
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
                    .and_then(ID::from_hex)
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

pub fn deserialize<R: Read>(mut read: R) -> io::Result<Object> {
    let obj = read_item(&mut read)?;
    if read.read(&mut [0u8])? != 0 {
        invalid!("trailing bytes");
    }
    let mut obj = match obj {
        Item::Dict(d) => d,
        _ => {
            invalid!("not a dict");
        }
    };
    if obj.keys().collect::<Vec<_>>() != &["d", "h", "i", "r"] {
        invalid!("unknown keys in dict");
    }
    if obj.get("d").and_then(Item::str) != Some("dhstore_0001") {
        invalid!("unknown format");
    }
    let id = match obj.get("h").and_then(Item::str)
        .map(str::as_bytes).and_then(ID::from_hex)
    {
        Some(id) => id,
        None => invalid!("invalid ID"),
    };
    let data = match (obj.remove("i").as_ref().and_then(Item::str),
                      obj.remove("r").unwrap()) {
        (Some("o"), Item::Dict(d)) => {
            let mut dict = BTreeMap::new();
            for (k, v) in d.into_iter() {
                match convert_property(v) {
                    Some(v) => { dict.insert(k, v); }
                    None => invalid!("invalid dict value"),
                }
            }
            ObjectData::Dict(dict)
        }
        (Some("o"), _) => invalid!("object is 'o' but not a dict"),
        (Some("l"), Item::List(l)) => {
            let mut list =  Vec::new();
            for v in l.into_iter() {
                match convert_property(v) {
                    Some(v) => list.push(v),
                    None => invalid!("invalid list value"),
                }
            }
            ObjectData::List(list)
        }
        (Some("l"), _) => invalid!("object is 'l' but not a list"),
        (Some(i), _) => invalid!("unknown object type '{}'", i),
        _ => invalid!("invalid object type"),
    };
    Ok(Object {
        id: id,
        data: data,
    })
}

pub fn hash_object(data: ObjectData) -> Object {
    let mut hasher = Hasher::new();
    write_data(&mut hasher, &data).unwrap();
    Object {
        id: hasher.result(),
        data: data,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::io::{Cursor, Write};

    use common::{ID, ObjectData, Property};
    use hash::Hasher;
    use serialize::{Item, hash_object, read_item, serialize, deserialize};

    fn fake_id(digit: u8) -> ID {
        ID::from_hex(&[b'0' + digit as u8; 64]).unwrap()
    }

    const test_dict: &[u8] =
        b"d\
          1:d12:dhstore_0001\
          1:h64:ed2f5d00a27066ea63ae8ddffb58e0c7\
          f28f4b8620784dc12e3fbcb01c52f79a\
          1:i1:o\
          1:rd\
          6:camera\
          d3:ref64:11111111111111111111111111111111\
          11111111111111111111111111111111e\
          4:data\
          d4:blob64:22222222222222222222222222222222\
          22222222222222222222222222222222e\
          8:filename\
          22:DSC_20170303223104.jpg\
          6:people\
          i5e\
          ee";

    #[test]
    fn test_serialize_dict() {
        // Create properties
        let mut properties = BTreeMap::new();
        properties.insert("filename".into(),
                          Property::String("DSC_20170303223104.jpg".into()));
        properties.insert("people".into(), Property::Integer(5));
        properties.insert("camera".into(), Property::Reference(fake_id(1)));
        properties.insert("data".into(), Property::Blob(fake_id(2)));
        let hash = ID::from_hex(b"ed2f5d00a27066ea63ae8ddffb58e0c7\
                                  f28f4b8620784dc12e3fbcb01c52f79a").unwrap();
        let obj = hash_object(ObjectData::Dict(properties));
        assert_eq!(obj.id, hash);
        let mut serialized = Vec::new();
        serialize(&mut serialized, &obj).unwrap();
        assert_eq!(serialized, test_dict);
    }

    #[test]
    fn test_deserialize_dict() {
        let obj = deserialize(Cursor::new(test_dict)).unwrap();
    }

    const test_list: &[u8] =
        b"d\
          1:d12:dhstore_0001\
          1:h64:1875c2ab1a6ee9d1bd9ee4b4f70ea819\
          cddb97ed3f15d5a24a3fd08c61b96407\
          1:i1:l\
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
        let mut properties: Vec<Property> = ["cvs", "subversion", "darcs", "git", "mercurial"]
            .iter()
            .map(|&s: &&str| -> String { s.into() })
            .map(Property::String)
            .collect();
        let hash = ID::from_hex(b"1875c2ab1a6ee9d1bd9ee4b4f70ea819\
                                  cddb97ed3f15d5a24a3fd08c61b96407").unwrap();
        let obj = hash_object(ObjectData::List(properties));
        assert_eq!(obj.id, hash);
        let mut serialized = Vec::new();
        serialize(&mut serialized, &obj).unwrap();
        assert_eq!(serialized, test_list);
    }

    #[test]
    fn test_deserialize_list() {
        let obj = deserialize(Cursor::new(test_list)).unwrap();
    }
}
