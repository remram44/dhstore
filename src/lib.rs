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

use std::fs::File;
use std::io::Read;

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
