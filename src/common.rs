//! # Common DHStore definitions
//!
//! This module contains the basic structs `Object`, `Property`, and the
//! `BlobStorage` and `ObjectIndex` traits.

use std::collections::{BTreeMap, HashSet};

use errors;
pub use hash::ID;

/// Values that appear in an object's metadata.
///
/// This is either an integer, a string, or a reference to another object.
#[derive(Debug, PartialEq, Eq)]
pub enum Property {
    String(String),
    Integer(i64),
    Reference(ID),
    Blob(ID),
}

pub type Dict = BTreeMap<String, Property>;
pub type List = Vec<Property>;

/// The types of object known to the index.
///
/// Object is simply this structure with an `ID` tacked on.
pub enum ObjectData {
    Dict(Dict),
    List(List),
}

/// A schema object, i.e. either a dictionary or a list of properties.
pub struct Object {
    pub id: ID,
    pub data: ObjectData,
}

/// Trait for the blob storage backends, that handle the specifics of storing
/// blobs. A blob is an unnamed sequence of bytes, which constitute parts of
/// some file's contents.
pub trait BlobStorage {
    /// Gets a blob from its ID.
    fn get_blob(&self, id: &ID) -> errors::Result<Option<Box<[u8]>>>;
    /// Hashes a blob then adds it to the store.
    fn add_blob(&mut self, blob: &[u8]) -> errors::Result<ID>;
    /// Adds a blob whose hash is already known.
    fn add_known_blob(&mut self, id: &ID, blob: &[u8]) -> errors::Result<()>;
    /// Deletes a blob from its hash.
    fn delete_blob(&mut self, id: &ID) -> errors::Result<()>;
    /// Checks the blob storage for errors.
    fn verify(&mut self) -> errors::Result<()>;
}

/// Additional trait for a `BlobStorage` that knows how to enumerate all the
/// blobs it has.
pub trait EnumerableBlobStorage: BlobStorage {
    type Iter: Iterator<Item = errors::Result<ID>>;

    /// Returns an iterator over the blobs in this store.
    fn list_blobs(&self) -> errors::Result<Self::Iter>;
    /// Removes the blobs whose hash are not in the given set.
    fn collect_garbage(&mut self, alive: HashSet<ID>) -> errors::Result<()> {
        for blob in self.list_blobs()? {
            let blob = blob?;
            if alive.contains(&blob) {
                self.delete_blob(&blob)?;
            }
        }
        Ok(())
    }
}

/// Trait for the index of schema objects.
///
/// This is a sort of database that can store `Object`s and knows how to make
/// sense of them and query them efficiently.
pub trait ObjectIndex {
    /// Hashes an object and adds it to the index.
    fn add(&mut self, data: ObjectData) -> errors::Result<ID>;
    /// Gets an object from its hash.
    fn get_object(&self, id: &ID) -> errors::Result<Option<&Object>>;
    /// Checks the index for errors.
    fn verify(&mut self) -> errors::Result<()>;
    /// Deletes unreferenced objects and returns the set of blobs to keep.
    fn collect_garbage(&mut self) -> errors::Result<HashSet<ID>>;
}

pub struct Query {
    key: String,
    value: Comparison,
}

pub enum Comparison {
    Equal(String),
    Like(String),
    Range(Option<i64>, Option<i64>),
    And(Vec<Comparison>),
    Or(Vec<Comparison>),
}

pub struct Path {
    root: ID,
    components: Vec<PathComponent>,
}

pub enum PathComponent {
    Id(ID),
    Query(Query),
}

pub trait Cursor {
    fn next(&mut self) -> Object;
    fn ignore(&mut self, id: &ID);
}
