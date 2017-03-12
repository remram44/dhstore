extern crate chunker;
#[macro_use]
extern crate log as log_crate;
extern crate sha2;
extern crate termcolor;

mod common;
pub mod errors;
mod file_storage;
pub mod hash;
pub mod log;
mod memory_index;
mod serialize;

use chunker::{ChunkInput, chunks};
pub use common::{ID, Property, ObjectData, Object,
                 BlobStorage, EnumerableBlobStorage, ObjectIndex};
use errors::Error;
pub use memory_index::MemoryIndex;
pub use file_storage::FileBlobStorage;

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

/// Main structure, representing the whole system.
pub struct Store<S: BlobStorage, I: ObjectIndex> {
    storage: S,
    index: I,
}

fn indent(level: usize) {
    for _ in 0..level {
        print!("  ");
    }
}

impl<S: BlobStorage, I: ObjectIndex> Store<S, I> {
    pub fn new(storage: S, index: I) -> Store<S, I> {
        Store {
            storage: storage,
            index: index,
        }
    }

    pub fn add_blob<R: Read>(&mut self, mut reader: R) -> errors::Result<ID> {
        let mut blob = Vec::new();
        reader.read_to_end(&mut blob).map_err(|e| ("Error reading blob", e))?;
        self.storage.add_blob(&blob)
    }

    pub fn get_blob(&self, id: &ID) -> errors::Result<Option<Box<[u8]>>> {
        self.storage.get_blob(id)
    }

    pub fn get_object(&self, id: &ID) -> errors::Result<Option<&Object>> {
        self.index.get_object(id)
    }

    pub fn add_file<R: Read>(&mut self, reader: R)
        -> errors::Result<(ID, usize)>
    {
        let mut blob = Vec::new();
        let mut iter = chunks(reader, 13); // 8 KiB average
        let mut chunks = Vec::new();
        let mut size = 0;
        while let Some(chunk) = iter.read() {
            let chunk = chunk.map_err(|e| ("Error reading from blob", e))?;
            match chunk {
                ChunkInput::Data(d) => blob.extend_from_slice(d),
                ChunkInput::End => {
                    size += blob.len();
                    let id = self.storage.add_blob(&blob)?;
                    chunks.push(Property::Blob(id));
                    blob.clear();
                }
            }
        }
        let nb_chunks = chunks.len();
        let id = self.index.add(ObjectData::List(chunks))?;
        info!("Added file contents, {} chunks, id = {}", nb_chunks, id);
        Ok((id, size))
    }

    pub fn add_dir<P: AsRef<Path>>(&mut self, path: P)
        -> errors::Result<ID>
    {
        let path = path.as_ref();
        let mut contents = BTreeMap::new();
        let entries = path.read_dir()
            .map_err(|e| ("Couldn't list directory to be added", e))?;
        for entry in entries {
            let entry = entry.map_err(|e| ("Error reading directory", e))?;
            let id = self.add(entry.path())?;
            contents.insert(entry.file_name().to_string_lossy().into_owned(),
                            Property::Reference(id));
        }
        let nb_entries = contents.len();
        let id = self.index.add(ObjectData::Dict(contents))?;
        info!("Added directory {:?}, {} entries, id = {}",
              path, nb_entries, id);
        Ok(id)
    }

    pub fn add<P: AsRef<Path>>(&mut self, path: P)
        -> errors::Result<ID>
    {
        let path = path.as_ref();
        if path.is_dir() {
            self.add_dir(path)
        } else if path.is_file() {
            let fp = File::open(path)
                .map_err(|e| ("Can't open file to be added", e))?;
            let (contents_id, size) = self.add_file(fp)?;
            let mut map = BTreeMap::new();
            map.insert("size".into(), Property::Integer(size as i64));
            map.insert("contents".into(),
                       Property::Reference(contents_id.clone()));
            let id = self.index.add(ObjectData::Dict(map))?;
            info!("Added file {:?}, size = {}, contents = {}, id = {}",
                  path, size, contents_id, id);
            Ok(id)
        } else {
            return Err(errors::Error::IoError("Can't find path to be added",
                                              io::ErrorKind::NotFound.into()));
        }
    }

    pub fn verify(&mut self) -> errors::Result<()> {
        info!("Verifying objects...");
        self.index.verify()?;
        info!("Verifying blobs...");
        self.storage.verify()
    }

    fn print_property(&self, property: &Property,
                      limit: Option<usize>,
                      level: usize)
        -> errors::Result<()>
    {
        match property {
            &Property::String(ref s) => print!("{:?}", s),
            &Property::Integer(i) => print!("{}", i),
            &Property::Reference(ref id) => {
                match self.get_object(id)? {
                    Some(obj) => self.print_obj_(obj, limit, level)?,
                    None => print!("{} #missing#", id),
                }
            }
            &Property::Blob(ref id) => print!("blob-{}", id),
        }
        Ok(())
    }

    fn print_obj_(&self, object: &Object,
                  limit: Option<usize>,
                  mut level: usize)
        -> errors::Result<()>
    {
        let recurse = limit.map_or(true, |l| level < l);

        if recurse {
            match object.data {
                ObjectData::Dict(ref dict) => {
                    println!("{} {{", object.id);
                    level += 1;
                    for (k, v) in dict {
                        indent(level);
                        print!("{:?} ", k);
                        self.print_property(v, limit, level)?;
                        println!();
                    }
                    level -= 1;
                    indent(level); print!("}}");
                }
                ObjectData::List(ref list) => {
                    println!("{} [", object.id);
                    level += 1;
                    for v in list {
                        indent(level);
                        self.print_property(v, limit, level)?;
                        println!();
                    }
                    level -= 1;
                    indent(level);
                    print!("]");
                }
            }
        } else {
            match object.data {
                ObjectData::Dict(_) => println!("{} {{ ... }}", object.id),
                ObjectData::List(_) => println!("{} [ ... ]", object.id),
            }
        }
        Ok(())
    }

    pub fn print_object(&self, id: &ID, limit: Option<usize>)
        -> errors::Result<()>
    {
        self.print_property(&Property::Reference(id.clone()), limit, 0)?;
        println!();
        Ok(())
    }
}

impl<S: EnumerableBlobStorage, I: ObjectIndex> Store<S, I> {
    pub fn collect_garbage(&mut self) -> errors::Result<()> {
        info!("Collecting objects...");
        let live_blobs = self.index.collect_garbage()?;
        info!("Collecting blobs...");
        self.storage.collect_garbage(live_blobs)
    }
}

pub fn open<P: AsRef<Path>>(path: P)
    -> errors::Result<Store<FileBlobStorage, MemoryIndex>>
{
    let path = path.as_ref();

    fs::metadata(path).map_err(|e| ("Store path doesn't exist", e))?;

    // Get the ID of the root config -- the configuration is loaded from the
    // index itself but we need a trust anchor
    let root_config = {
        let mut fp = File::open(path.join("root"))
            .map_err(|e| ("Can't open root config file", e))?;
        let mut buf = Vec::new();
        fp.read_to_end(&mut buf)
            .map_err(|e| ("Error reading root config file", e))?;
        ID::from_slice(&buf)
            .ok_or(Error::CorruptedStore("Invalid root config file"))?
    };

    // Create a file blob storage, storing blobs as single files
    let storage = {
        FileBlobStorage::open(path.join("blobs"))
    };

    // Create a memory index, that stores all the objects in memory, and
    // has to load all of them everytime from simple files
    let index = {
        MemoryIndex::open(path.join("objects"), root_config)?
    };

    // Create the Store object
    Ok(Store::new(storage, index))
}

pub fn create<P: AsRef<Path>>(path: P) -> errors::Result<()> {
    let path = path.as_ref();

    // Create directory
    if path.is_dir() {
        if path.read_dir()
                .map_err(|e| ("Couldn't list target directory", e))?
                .next().is_some() {
            return Err(Error::CorruptedStore(
                "Target directory exists and is not empty"));
        }
    } else if path.exists() {
        return Err(Error::CorruptedStore(
            "Target exists and is not a directory"));
    } else {
        ::std::fs::create_dir(path)
            .map_err(|e| ("Couldn't create directory", e))?;
    }

    // Create blobs directory
    ::std::fs::create_dir(path.join("blobs"))
        .map_err(|e| ("Couldn't create directory", e))?;

    // Create objects directory
    ::std::fs::create_dir(path.join("objects"))
        .map_err(|e| ("Couldn't create directory", e))?;

    // Create root config
    {
        let mut fp = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path.join("root"))
            .map_err(|e| ("Couldn't open root config", e))?;
        // TODO : Write root config
        fp.write_all(b"\x00\x01\x02\x03\x04\x05\x06\x07\
                       \x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\
                       \x10\x11\x12\x13\x14\x15\x16\x17\
                       \x18\x19\x1a\x1b\x1c\x1d\x1e\x1f")
            .map_err(|e| ("Couldn't write root config", e))?;
    }

    Ok(())
}
