//! Implementation of an object indexer that stores everything in memory.
//!
//! This loads all the objects from disk into memory. Objects added to the index
//! are immediately written to disk as well.
//!
//! This is very inefficient and should be backed by proper database code at
//! some point.

use log_crate::LogLevel;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{PathBuf, Path};

use common::{ID, Object, ObjectData, ObjectIndex, Property};
use errors::{self, Error};
use serialize;

/// Return value from a Policy for some object.
pub enum PolicyDecision {
    Get,
    Keep,
    Drop,
}

/// A policy defines which objects are valid and which we want to keep.
///
/// DHStore has a root configuration where the user defines what trees he wants
/// to keep, schemas to validate those trees against, disk usage limits, etc. He
/// can also set up delegations, to read in more policy objects recursively, ...
///
/// A `Policy` object contains all this information for a specific place in the
/// tree, and handles all the builtin, user-supplied, and recursive behaviors
/// for the index.
pub trait Policy {
    fn handle(&mut self, property: &str, object: Object)
              -> (PolicyDecision, Box<Policy>);
}

/// Placeholder Policy that keeps everything.
struct KeepPolicy;

impl KeepPolicy {
    fn new() -> KeepPolicy {
        KeepPolicy
    }
}

impl Policy for KeepPolicy {
    fn handle(&mut self, property: &str, object: Object)
              -> (PolicyDecision, Box<Policy>) {
        (PolicyDecision::Keep, Box::new(KeepPolicy))
    }
}

/// Key of a reference, used in the backward reference map.
///
/// A reference is a value, and can appear in both types of schema objects: in a
/// dict, it is associated with a string key, and in a list, with an index.
#[derive(PartialEq, Eq, Hash)]
enum Backkey {
    Index(usize),
    Key(String),
}

/// The in-memory index, that loads all objects from the disk on startup.
pub struct MemoryIndex {
    path: PathBuf,
    objects: HashMap<ID, Object>,
    backlinks: HashMap<ID, HashSet<(Backkey, ID)>>,
    root: ID,
    log: Option<ID>,
    policy: Box<Policy>,
}

impl MemoryIndex {
    /// Reads all the objects from a directory into memory.
    pub fn open<P: AsRef<Path>>(path: P, root: ID)
        -> errors::Result<MemoryIndex>
    {
        let path = path.as_ref();
        let mut index = MemoryIndex {
            path: path.to_path_buf(),
            objects: HashMap::new(),
            backlinks: HashMap::new(),
            root: root.clone(),
            log: None,
            policy: Box::new(KeepPolicy::new()),
        };
        let dirlist = path.read_dir()
            .map_err(|e| ("Error listing objects directory", e))?;
        for first in dirlist {
            let first = first
                .map_err(|e| ("Error listing objects directory", e))?;
            let dirlist = first.path().read_dir()
                .map_err(|e| ("Error listing objects subdirectory", e))?;
            for second in dirlist {
                let second = second
                    .map_err(|e| ("Error listing objects subdirectory", e))?;
                let filename = second.path();

                // Read object
                let fp = File::open(filename)
                    .map_err(|e| ("Error opening object", e))?;
                let object = match serialize::deserialize(fp) {
                    Err(e) => {
                        let mut path: PathBuf = first.file_name().into();
                        path.push(second.file_name());
                        error!("Error deserializing object: {:?}", path);
                        return Err(("Error deserializing object", e).into());
                    }
                    Ok(o) => o,
                };

                index.insert_object_do_backrefs(object);
            }
        }

        // Parse root config
        index.log = {
            let config = index.get_object(&root)?
                .ok_or(Error::CorruptedStore("Missing root object"))?;
            let config = match config.data {
                ObjectData::Dict(ref dict) => dict,
                _ => return Err(Error::CorruptedStore(
                    "Root object is not a dict")),
            };
            match config.get("log") {
                Some(&Property::Reference(ref id)) => {
                    let log_obj = index.get_object(id)?
                        .ok_or(Error::CorruptedStore("Missing log object"))?;
                    match log_obj.data {
                        ObjectData::Dict(_) => {
                            debug!("Activated log: {}", id);
                        }
                        _ => {
                            return Err(Error::CorruptedStore(
                                "Log is not a permanode"));
                        }
                    }
                    Some(id.clone())
                }
                Some(_) => return Err(Error::CorruptedStore(
                    "Log is not a reference")),
                None => None,
            }
        };

        Ok(index)
    }

    pub fn create<'a, P: AsRef<Path>, I: Iterator<Item=&'a Object>>(
            path: P, objects: I)
        -> io::Result<()>
    {
        for object in objects {
            MemoryIndex::write_object(path.as_ref(), object)?;
        }
        Ok(())
    }

    fn write_object(dir: &Path, object: &Object) -> io::Result<()> {
        let hashstr = object.id.str();
        let mut path = dir.join(&hashstr[..4]);
        if !path.exists() {
            fs::create_dir(&path)?;
        }
        path.push(&hashstr[4..]);
        let mut fp = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        serialize::serialize(&mut fp, object)
    }

    /// Utility to insert a new object in the store.
    ///
    /// Insert the object, indexing the back references.
    fn insert_object_do_backrefs(&mut self, object: Object) {
        // Record reverse references
        {
            let mut insert = |target: &ID, key: Backkey, source: ID| {
                if log_enabled!(LogLevel::Debug) {
                    match key {
                        Backkey::Index(i) => {
                            debug!("Reference {} -> {} ({})",
                                   source, target, i);
                        }
                        Backkey::Key(ref k) => {
                            debug!("Reference {} -> {} ({})",
                                   source, target, k);
                        }
                    }
                }

                // Add backlink
                if let Some(set) = self.backlinks.get_mut(target) {
                    set.insert((key, source));
                    return;
                }
                let mut set = HashSet::new();
                set.insert((key, source));
                self.backlinks.insert(target.clone(), set);
            };
            match object.data {
                ObjectData::Dict(ref dict) => {
                    for (k, v) in dict {
                        if let Property::Reference(ref id) = *v {
                            insert(id,
                                   Backkey::Key(k.clone()),
                                   object.id.clone());
                        }
                    }
                }
                ObjectData::List(ref list) => {
                    for (k, v) in list.into_iter().enumerate() {
                        if let Property::Reference(ref id) = *v {
                            insert(id,
                                   Backkey::Index(k),
                                   object.id.clone());
                        }
                    }
                }
            }
        }

        self.objects.insert(object.id.clone(), object);
    }

    /// Common logic for `verify()` and `collect_garbage().`
    ///
    /// Goes over the tree of objects, checking for errors. If `collect` is
    /// true, unreferenced objects are deleted, and the set of referenced blobs
    /// is returned; else, an empty `HashSet` is returned.
    fn walk(&mut self, collect: bool) -> errors::Result<HashSet<ID>> {
        let mut alive = HashSet::new(); // ids
        let mut live_blobs = HashSet::new(); // ids
        let mut open = VecDeque::new(); // ids
        if self.objects.get(&self.root).is_none() {
            error!("Root is missing: {}", self.root);
        } else {
            open.push_front(self.root.clone());
        }
        while let Some(id) = open.pop_front() {
            debug!("Walking, open={}, alive={}/{}, id={}",
                   open.len(), alive.len(), self.objects.len(), id);
            let object = match self.objects.get(&id) {
                Some(o) => o,
                None => {
                    info!("Don't have object {}", id);
                    continue;
                }
            };
            if alive.contains(&id) {
                debug!("  already alive");
                continue;
            }
            alive.insert(id);
            let mut handle = |value: &Property| {
                match *value {
                    Property::Reference(ref id) => {
                        open.push_back(id.clone());
                    }
                    Property::Blob(ref id) => {
                        if collect {
                            live_blobs.insert(id.clone());
                        }
                    }
                    _ => {}
                }
            };
            match object.data {
                ObjectData::Dict(ref dict) => {
                    debug!("  is dict, {} values", dict.len());
                    for v in dict.values() {
                        handle(v);
                    }
                }
                ObjectData::List(ref list) => {
                    debug!("  is list, {} values", list.len());
                    for v in list {
                        handle(v);
                    }
                }
            }
        }
        info!("Found {}/{} live objects", alive.len(), self.objects.len());
        if collect {
            let dead_objects = self.objects.keys()
                .filter(|id| !alive.contains(id))
                .cloned()
                .collect::<Vec<_>>();
            info!("Removing {} dead objects", dead_objects.len());
            for id in dead_objects {
                self.objects.remove(&id);
            }
        }
        Ok(live_blobs)
    }
}

impl ObjectIndex for MemoryIndex {
    fn add(&mut self, data: ObjectData) -> errors::Result<ID> {
        let object = serialize::hash_object(data);
        let id = object.id.clone();
        if !self.objects.contains_key(&id) {
            info!("Adding object to index: {}", id);
            MemoryIndex::write_object(&self.path, &object)
                .map_err(|e| ("Couldn't write object to disk", e))?;
            self.insert_object_do_backrefs(object);
        }
        Ok(id)
    }

    fn get_object(&self, id: &ID) -> errors::Result<Option<&Object>> {
        Ok(self.objects.get(id))
    }

    fn verify(&mut self) -> errors::Result<()> {
        self.walk(false).map(|_| ())
    }

    fn collect_garbage(&mut self) -> errors::Result<HashSet<ID>> {
        self.walk(true)
    }
}
