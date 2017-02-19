#[macro_use]
extern crate log;
extern crate sha1;

mod common;
mod memory_index;
mod file_storage;

pub use common::{ID, Property, Object, Path, PathComponent, BlobStorage,
                 EnumerableBlobStorage, ObjectIndex};
pub use memory_index::MemoryIndex;
pub use file_storage::FileBlobStorage;

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

/// Main structure, representing the whole system.
pub struct Store<S: BlobStorage, I: ObjectIndex> {
    storage: S,
    index: I,
    root_config: ID,
}

impl<S: BlobStorage, I: ObjectIndex> Store<S, I> {
    pub fn new(storage: S, index: I, root_config: ID) -> Store<S, I> {
        Store {
            storage: storage,
            index: index,
            root_config: root_config,
        }
    }
}

pub fn open<P: AsRef<::std::path::Path>>(path: P)
     -> Store<FileBlobStorage, MemoryIndex> {
    let path = path.as_ref();

    // Create a file blob storage, storing blobs as single files
    let storage = {
        FileBlobStorage::open(path.join("blobs"))
    };

    // Create a memory index, that stores all the objects in memory, and
    // has to load all of them everytime from simple files
    let index = {
        MemoryIndex::open(path.join("objects"))
    };

    // Get the ID of the root config -- the configuration is loaded from the
    // index itself but we need a trust anchor
    let root_config = {
        let mut fp = File::open(path.join("root"))
            .expect("Can't open root config file");
        let mut buf = Vec::new();
        fp.read_to_end(&mut buf).expect("Error reading root config file");
        if buf.len() != 20 {
            panic!("Invalid root config file");
        }
        let mut bytes = [0u8; 20];
        bytes.clone_from_slice(&buf);
        ID { bytes: bytes }
    };

    // Create the Store object
    Store::new(storage, index, root_config)
}

pub fn create<P: AsRef<::std::path::Path>>(path: P) {
    let path = path.as_ref();

    // Create directory
    if path.is_dir() {
        if path.read_dir()
            .expect("Couldn't list target directory")
            .next().is_some() {
            panic!("Target directory exists and is not empty");
        }
    } else if path.exists() {
        panic!("Target exists and is not a directory");
    } else {
        ::std::fs::create_dir(path);
    }

    // Create blobs directory
    ::std::fs::create_dir(path.join("blobs")).unwrap();

    // Create objects directory
    ::std::fs::create_dir(path.join("objects")).unwrap();

    // Create root config
    {
        let mut fp = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path.join("root"))
            .unwrap();
        fp.write_all(b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\
                       \x0a\x0b\x0c\x0d\x0e\x0f\x10\x11\x12\x13")
            .unwrap();
    }
}
