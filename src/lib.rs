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
}

pub fn open<P: AsRef<std::path::Path>>(path: P) ->
    Store<FileBlobStorage, MemoryIndex>
{
    let path = path.as_ref();

    // Create a file blob storage, storing blobs as single files
    let storage = {
        let mut blobs = path.to_path_buf();
        blobs.push("blobs");
        FileBlobStorage::open(blobs)
    };

    // Create a memory index, that stores all the objects in memory, and
    // has to load all of them everytime from simple files
    let index = {
        let mut objects = path.to_path_buf();
        objects.push("objects");
        MemoryIndex::open(objects)
    };

    // Create the Store object
    Store::new(storage, index)
}
