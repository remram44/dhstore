//! DHStore: A personal content management system.

extern crate chunker;
#[macro_use]
extern crate log as log_crate;
extern crate rand;
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
pub use common::{HASH_SIZE, ID, Dict, List, Property, ObjectData, Object,
                 BlobStorage, EnumerableBlobStorage, ObjectIndex};
use errors::Error;
pub use memory_index::MemoryIndex;
pub use file_storage::FileBlobStorage;

use rand::Rng;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::mem::swap;
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
    /// Creates a store from a given blob storage and object index.
    pub fn new(storage: S, index: I) -> Store<S, I> {
        Store {
            storage: storage,
            index: index,
        }
    }

    /// Low-level; adds a blob to the blob storage.
    ///
    /// To cut a blob into chunks, add them to the blob storage, and return a
    /// list object of them, use `Store::add_file()`.
    pub fn add_blob<R: Read>(&mut self, mut reader: R) -> errors::Result<ID> {
        let mut blob = Vec::new();
        reader.read_to_end(&mut blob).map_err(|e| ("Error reading blob", e))?;
        self.storage.add_blob(&blob)
    }

    /// Low-level; gets a single blob from the blob storage.
    pub fn get_blob(&self, id: &ID) -> errors::Result<Option<Box<[u8]>>> {
        self.storage.get_blob(id)
    }

    /// Low-level; gets a single object from the index by its ID.
    pub fn get_object(&self, id: &ID) -> errors::Result<Option<&Object>> {
        self.index.get_object(id)
    }

    /// Cuts a file into chunks and add a list object of them to the index.
    pub fn add_file<R: Read>(&mut self, reader: R)
        -> errors::Result<(ID, usize)>
    {
        let mut blob = Vec::new();
        let mut iter = chunks(reader, 13); // 8 KiB average
        let mut chunks = Vec::new();
        let mut size = 0;
        const MAX_LEN: usize = 64 * 1024; // 64 KiB hard maximum
        while let Some(chunk) = iter.read() {
            let chunk = chunk.map_err(|e| ("Error reading from blob", e))?;
            match chunk {
                ChunkInput::Data(d) => {
                    blob.extend_from_slice(d);
                    if blob.len() > MAX_LEN {
                        let mut other = blob.split_off(MAX_LEN);
                        swap(&mut blob, &mut other);
                        assert_eq!(other.len(), MAX_LEN);
                        size += other.len();
                        let id = self.storage.add_blob(&other)?;
                        chunks.push(Property::Blob(id));
                    }
                }
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

    fn add_dir<P: AsRef<Path>>(&mut self, path: P)
        -> errors::Result<ID>
    {
        let path = path.as_ref();
        let mut contents = Dict::new();
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

    /// Adds a file or directory recursively, representing directories as dicts
    /// and files as lists of blobs.
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
            let mut map = Dict::new();
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

    /// Checks the blobs and objects for errors.
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
        match *property {
            Property::String(ref s) => print!("{:?}", s),
            Property::Integer(i) => print!("{}", i),
            Property::Reference(ref id) => {
                match self.get_object(id)? {
                    Some(obj) => self.print_obj_(obj, limit, level)?,
                    None => print!("{} #missing#", id),
                }
            }
            Property::Blob(ref id) => print!("blob-{}", id),
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

    /// Pretty-prints objects recursively.
    ///
    /// If `limit` is not `None`, it is the maximum depth of nested objects
    /// we'll print; for example, `Some(1)` means that objects directly
    /// referenced from the given one will be expanded, but not objects
    /// referenced from those.
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

pub fn permanode(mut data: Dict) -> Object {
    let mut random = [0u8; HASH_SIZE];
    rand::thread_rng().fill_bytes(&mut random);
    let random = ID::from_slice(&random).unwrap().str();
    data.insert("random".into(), Property::String(random));
    serialize::hash_object(ObjectData::Dict(data))
}

/// Opens a directory.
///
/// This uses the `FileBlobStorage` and `MemoryIndex` to create a `Store` from a
/// filesystem directory.
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
        ID::from_str(&buf)
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

/// Creates a new store on disk.
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

        // Log permanode
        let mut log = Dict::new();
        log.insert("type".into(), Property::String("set".into()));
        log.insert("order".into(), Property::String("date".into()));
        let log = permanode(log);

        // Config object
        let mut config = Dict::new();
        config.insert("log".into(), Property::Reference(log.id.clone()));
        let config = serialize::hash_object(ObjectData::Dict(config));
        let config_id = config.id.str();

        MemoryIndex::create(path.join("objects"), vec![log, config].iter())
            .map_err(|e| ("Couldn't write objects", e))?;

        // Write root config
        fp.write_all(config_id.as_bytes())
            .map_err(|e| ("Couldn't write root config", e))?;
    }

    Ok(())
}
