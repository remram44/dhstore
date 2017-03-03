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

pub use common::{ID, Property, Object, Path, PathComponent, BlobStorage,
                 EnumerableBlobStorage, ObjectIndex};
use errors::Error;
pub use memory_index::MemoryIndex;
pub use file_storage::FileBlobStorage;

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};

/// Main structure, representing the whole system.
pub struct Store<S: BlobStorage, I: ObjectIndex> {
    storage: S,
    index: I,
}

impl<S: BlobStorage, I: ObjectIndex> Store<S, I> {
    pub fn new(storage: S, index: I) -> Store<S, I> {
        Store {
            storage: storage,
            index: index,
        }
    }

    pub fn add_blob<R: Read>(&mut self, blob: R) -> errors::Result<ID> {
        self.storage.copy_blob(blob)
    }

    pub fn verify(&mut self) -> errors::Result<()> {
        info!("Verifying objects...");
        self.index.verify(false)?;
        info!("Verifying blobs...");
        self.storage.verify()
    }
}

pub fn open<P: AsRef<::std::path::Path>>(path: P)
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

pub fn create<P: AsRef<::std::path::Path>>(path: P) -> errors::Result<()> {
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
        // TODO
        fp.write_all(b"\x00\x01\x02\x03\x04\x05\x06\x07\
                       \x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\
                       \x10\x11\x12\x13\x14\x15\x16\x17\
                       \x18\x19\x1a\x1b\x1c\x1d\x1e\x1f")
            .map_err(|e| ("Couldn't write root config", e))?;
    }

    Ok(())
}
