use std::collections::HashMap;
use std::io::Read;

use errors;
pub use hash::ID;

/// Values that appear in an object's metadata.
///
/// This is either an integer, a string, or a reference to another object.
pub enum Property {
    String(String),
    Integer(i64),
    Reference(ID),
    Blob(ID),
}

/// A schema object, i.e. a dictionary of properties.
pub struct Object {
    pub id: ID,
    pub properties: HashMap<String, Property>,
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

pub trait BlobStorage {
    fn get_blob(&self, id: &ID) -> errors::Result<Option<Box<[u8]>>>;
    fn add_blob(&mut self, blob: &[u8]) -> errors::Result<ID>;
    fn add_known_blob(&mut self, id: &ID, blob: &[u8]) -> errors::Result<()>;
    fn copy_blob<R: Read>(&mut self, blob: R) -> errors::Result<ID>;
    fn delete_blob(&mut self, id: &ID) -> errors::Result<()>;
    fn verify(&mut self) -> errors::Result<()>;
}

pub trait EnumerableBlobStorage: BlobStorage {
    type Iter: Iterator<Item = errors::Result<ID>>;

    fn list_blobs(&self) -> errors::Result<Self::Iter>;
}

pub trait ObjectIndex {
    fn verify(&mut self, collect: bool) -> errors::Result<()>;
}

pub trait Cursor {
    fn next(&mut self) -> Object;
    fn ignore(&mut self, id: &ID);
}

impl PartialEq for ID {
    fn eq(&self, other: &ID) -> bool {
        self.bytes == other.bytes
    }
}
