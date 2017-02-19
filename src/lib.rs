#[macro_use]
extern crate log;
extern crate sha1;

mod common;
mod memory_indexer;
mod file_storage;

pub use common::{ID, Property, Object, Path, PathComponent, BlobStorage,
                 EnumerableBlobStorage, ObjectIndex};
pub use memory_indexer::MemoryIndex;
pub use file_storage::FileBlobStorage;

/// Main structure, representing the whole system.
pub struct Store<S: BlobStorage, I: ObjectIndex> {
    storage: S,
    index: I,
}

impl<S: BlobStorage, I: ObjectIndex> Store<S, I> {
    pub fn open(storage: S, index: I) -> Store<S, I> {
        Store {
            storage: storage,
            index: index,
        }
    }
}
