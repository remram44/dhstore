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
use std::path::{PathBuf, Path};

use common::{ID, Object, ObjectData, ObjectIndex, Property};
use errors;
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

/// Placeholder. Policy is not yet implemented.
struct PolicyV1;

impl PolicyV1 {
    fn new() -> PolicyV1 {
        PolicyV1
    }
}

impl Policy for PolicyV1 {
    fn handle(&mut self, property: &str, object: Object)
              -> (PolicyDecision, Box<Policy>) {
        unimplemented!() // TODO: policy stuff
    }
}

/// An object with a reference count attached.
///
/// Because the index has mutable objects (permanodes), cycles are possible and
/// a proper garbage collector is necessary. However, for non-mutable nodes, a
/// reference count is still stored to make it faster to collect.
pub enum RefCount {
    Number(usize),
    Special,
}

/// An object with a reference count tacked on.
pub struct RefCountedObject {
    refs: RefCount,
    object: Object,
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
    objects: HashMap<ID, RefCountedObject>,
    backlinks: HashMap<ID, HashSet<(Backkey, ID)>>,
    root: ID,
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
            root: root,
            policy: Box::new(PolicyV1::new()),
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

        // TODO: Parse root config

        Ok(index)
    }

    /// Utility to insert a new object in the store while taking care of refs.
    ///
    /// Insert the object, sets its reference count from the back reference map,
    /// updates the reference count of referenced object, and adds references to
    /// the back reference map.
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

                // Increment refs of target
                if let Some(refobj) = self.objects.get_mut(target) {
                    match refobj.refs {
                        RefCount::Number(ref mut nb) => *nb += 1,
                        RefCount::Special => {}
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
                        match v {
                            &Property::Reference(ref id) => {
                                insert(id,
                                       Backkey::Key(k.clone()),
                                       object.id.clone());
                            }
                            _ => {}
                        }
                    }
                }
                ObjectData::List(ref list) => {
                    for (k, v) in list.into_iter().enumerate() {
                        match v {
                            &Property::Reference(ref id) => {
                                insert(id,
                                       Backkey::Index(k),
                                       object.id.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let refs = self.backlinks.get(&object.id)
            .map_or(0, |backlinks| backlinks.len());
        self.objects.insert(object.id.clone(),
                            RefCountedObject { refs: RefCount::Number(refs),
                                               object: object });
    }

    /// Common logic for `verify()` and c`ollect_garbage().`
    ///
    /// Goes over the tree of objects, checking for errors. If `collect` is
    /// true, unreferenced objects are deleted, and the set of referenced blobs
    /// is returned; else, an empty `HashSet` is returned.
    fn walk(&mut self, collect: bool) -> errors::Result<HashSet<ID>> {
        let mut alive = HashMap::new(); // ids => refcount
        let mut live_blobs = HashSet::new(); // ids
        let mut open = VecDeque::new(); // objects
        match self.objects.get(&self.root) {
            None => error!("Root is missing: {}", self.root),
            Some(obj) => open.push_front(obj),
        }
        while let Some(object) = open.pop_front() {
            let id = &object.object.id;
            debug!("Walking, open={}, alive={}/{}, id={}",
                   open.len(), alive.len(), self.objects.len(), id);
            if let Some(v) = alive.get_mut(id) {
                *v += 1;
                debug!("  already alive, incrementing refs to {}", v);
                continue;
            }
            alive.insert(id, 1);
            let mut handle = |value: &Property| {
                match value {
                    &Property::Reference(ref id) => {
                        if let Some(obj) = self.objects.get(id){
                            open.push_back(obj);
                        }
                    }
                    &Property::Blob(ref id) => {
                        if collect {
                            live_blobs.insert(id.clone());
                        }
                    }
                    _ => {}
                }
            };
            match object.object.data {
                ObjectData::Dict(ref dict) => {
                    debug!("  is dict, {} values", dict.len());
                    for (_, v) in dict {
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
            // TODO: Collect dead objects
            unimplemented!()
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
            let hex = id.hex();
            let mut path = self.path.join(&hex[..]);
            if !path.exists() {
                fs::create_dir(&path)
                    .map_err(|e| ("Can't create object directory", e))?;
            }
            path.push(&hex[2..]);
            let mut fp = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|e| ("Can't open object file", e))?;
            serialize::serialize(&mut fp, &object)
                .map_err(|e| ("Error writing object to disk", e))?;
            self.insert_object_do_backrefs(object);
        }
        Ok(id)
    }

    fn get_object(&self, id: &ID) -> errors::Result<Option<&Object>> {
        Ok(self.objects.get(id).map(|r| &r.object))
    }

    fn verify(&mut self) -> errors::Result<()> {
        self.walk(false).map(|_| ())
    }

    fn collect_garbage(&mut self) -> errors::Result<HashSet<ID>> {
        self.walk(true)
    }
}
