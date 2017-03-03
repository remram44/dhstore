use std::io::{self, Write};

use common::{ID, Object, Property};

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

pub fn serialize<W: Write>(out: &mut W, object: &Object) -> io::Result<()> {
    out.write_all(b"o")?;
    write_ref(out, &object.id)?;
    for (key, value) in &object.properties {
        write_str(out, key)?;
        write_property(out, value)?;
    }
    out.write_all(b"e")
}
