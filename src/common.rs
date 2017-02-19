use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

/// Identifier for an object.
///
/// Because they are content-addressable, this is a hash of its content.
pub struct ID {
    bytes: [u8; 20],
}

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
    fn get_blob(&self, id: &ID) -> Option<Box<[u8]>>;
    fn add_blob(&mut self, blob: &[u8]) -> ID;
    fn delete_blob(&mut self, id: &ID);
}

pub trait EnumerableBlobStorage: BlobStorage {
}

pub trait ObjectIndex {
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

impl Eq for ID {}

impl Hash for ID {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

impl Display for ID {
    fn fmt(&self, f: &mut Formatter) -> Result<(), ::std::fmt::Error> {
        for byte in self.bytes.iter() {
            write!(f, "{:020x}", byte)?;
        }
        Ok(())
    }
}
