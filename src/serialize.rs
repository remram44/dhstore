use std::collections::BTreeMap;
use std::io::{self, Read, Write};

use common::{ID, Object, Property};
use hash::Hasher;

// Object: o<id><key><value><key><value>...e
// String: 5:hello
// Integer: i42e
// Reference: r64:abcdef...
// Blob: b64:abcdef...

fn write_ref<W: Write>(out: &mut W, id: &ID) -> io::Result<()> {
    out.write_all(b"r")?;
    write_str(out, &id.hex())
}

fn write_str<W: Write>(out: &mut W, string: &str) -> io::Result<()> {
    write!(out, "{}:{}", string.len(), string)
}

fn write_property<W: Write>(out: &mut W, prop: &Property) -> io::Result<()> {
    match prop {
        &Property::String(ref s) => write_str(out, s),
        &Property::Integer(i) => write!(out, "i{}e", i),
        &Property::Reference(ref id) => write_ref(out, id),
        &Property::Blob(ref id) => {
            out.write_all(b"b")?;
            write_str(out, &id.hex())
        }
    }
}

fn write_properties<W: Write>(out: &mut W,
                              properties: &BTreeMap<String, Property>)
    -> io::Result<()>
{
    for (key, value) in properties {
        write_str(out, key)?;
        write_property(out, value)?;
    }
    Ok(())
}

pub fn serialize<W: Write>(out: &mut W, object: &Object) -> io::Result<()> {
    out.write_all(b"o")?;
    write_ref(out, &object.id)?;
    write_properties(out, &object.properties)?;
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

enum Item {
    Property(Property),
    End,
}

fn read_item<R: Read>(read: &mut R) -> io::Result<Item> {
    match read_byte(read)? {
        d @ b'0'...b'9' => {
            let mut len = (d - b'0') as usize;
            loop {
                let d = read_byte(read)?;
                if b'0' <= d && d <= b'9' {
                    len = len * 10 + (d - b'0') as usize;
                } else if d == b':' {
                    let mut s = String::new();
                    for _ in 0..len {
                        s.push(read_byte(read)? as char);
                    }
                    return Ok(Item::Property(Property::String(s)));
                } else {
                    return Err(io::ErrorKind::InvalidData.into());
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
                        return Err(io::ErrorKind::InvalidData.into());
                    }
                    let (n, o) = n.overflowing_add((d - b'0') as i64);
                    if o {
                        return Err(io::ErrorKind::InvalidData.into());
                    }
                    nb = n;
                } else if d == b'e' {
                    return Ok(Item::Property(Property::Integer(nb)));
                } else {
                    return Err(io::ErrorKind::InvalidData.into());
                }
            }
        }
        c @ b'r' | c @ b'b' => {
            if let Item::Property(Property::String(hex)) = read_item(read)? {
                if let Some(id) = ID::from_hex(hex.as_bytes()) {
                    return Ok(Item::Property(if c == b'r' {
                        Property::Reference(id)
                    } else {
                        Property::Blob(id)
                    }));
                }
            }
            Err(io::ErrorKind::InvalidData.into())
        }
        b'e' => Ok(Item::End),
        _ => Err(io::ErrorKind::InvalidData.into()),
    }
}

pub fn deserialize<R: Read>(mut read: R) -> io::Result<Object> {
    if read_byte(&mut read)? != b'o' {
        error!("deserialize: not an object");
        return Err(io::ErrorKind::InvalidData.into());
    }
    let id = match read_item(&mut read)? {
        Item::Property(Property::Reference(id)) => id,
        _ => {
            error!("deserialize: expected reference");
            return Err(io::ErrorKind::InvalidData.into());
        }
    };
    let mut properties = BTreeMap::new();
    loop {
        let key = match read_item(&mut read)? {
            Item::Property(Property::String(s)) => s,
            Item::End => break,
            _ => {
                error!("deserialize: expected string");
                return Err(io::ErrorKind::InvalidData.into());
            }
        };
        if properties.get(&key).is_some() {
            error!("deserialize: duplicate key");
            return Err(io::ErrorKind::InvalidData.into());
        }
        let value = match read_item(&mut read)? {
            Item::Property(prop) => prop,
            Item::End => {
                error!("deserialize: unexpected end of object");
                return Err(io::ErrorKind::InvalidData.into());
            }
        };
        properties.insert(key, value);
    }
    Ok(Object {
        id: id,
        properties: properties,
    })
}

pub fn hash_object(properties: BTreeMap<String, Property>) -> Object {
    let mut hasher = Hasher::new();
    write_properties(&mut hasher, &properties).unwrap();
    Object {
        id: hasher.result(),
        properties: properties,
    }
}
