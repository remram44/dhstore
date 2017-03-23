//! Implementation of an object indexer that stores everything in memory.
//!
//! This loads all the objects from disk into memory. Objects added to the index
//! are immediately written to disk as well.
//!
//! This is very inefficient and should be backed by proper database code at
//! some point.

use log_crate::LogLevel;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io;
use std::mem::swap;
use std::path::{PathBuf, Path};

use common::{HASH_SIZE, Sort, ID, Dict, Object, ObjectData, Property,
             ObjectIndex};
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
    /// Reference from a dict under this key.
    Key(String),
    /// Reference from a list from this index.
    Index(usize),
}

enum PermanodeType {
    Set,
    Single,
}

struct Permanode {
    sort: Sort,
    nodetype: PermanodeType,
    claims: BTreeMap<Property, ID>,
}

impl Permanode {
    fn index_claim(&mut self, claim: &Dict, permanode_id: &ID, claim_id: &ID) {
        // We require the claim to have the sort key
        let sort_value: &Property = match claim.get(self.sort.field()) {
            Some(ref prop) => prop,
            None => {
                debug!("Claim {} is invalid for permanode {}: \
                        missing sort key",
                       claim_id, permanode_id);
                return;
            }
        };
        // Currently, no validation is done; every claim is accepted
        // In the future, we'd have ways of checking a claim, such as public
        // key signatures (permanode has key, claim has signature)
        self.claims.insert(sort_value.clone(), claim_id.clone());
        match self.nodetype {
            PermanodeType::Set => {
                // Keep the whole set of values
                // TODO: handle set deletion claims
            }
            PermanodeType::Single => {
                // Keep one value, the latest by sorting order
                if self.claims.len() > 1 {
                    let mut map = BTreeMap::new();
                    swap(&mut self.claims, &mut map);
                    let mut map = map.into_iter();
                    let (k, v) = match self.sort {
                        Sort::Ascending(_) => map.next_back().unwrap(),
                        Sort::Descending(_) => map.next().unwrap(),
                    };
                    self.claims.insert(k, v);
                }
            }
        }
    }
}

fn insert_into_multimap<K: Clone + Eq + ::std::hash::Hash,
                        V: Eq + ::std::hash::Hash>(
    multimap: &mut HashMap<K, HashSet<V>>,
    key: &K,
    value: V)
{
    if let Some(set) = multimap.get_mut(key) {
        set.insert(value);
        return;
    }
    let mut set = HashSet::new();
    set.insert(value);
    multimap.insert(key.clone(), set);
}

/// The in-memory index, that loads all objects from the disk on startup.
pub struct MemoryIndex {
    /// Directory where objects are stored on disk.
    path: PathBuf,
    /// All objects, indexed by their ID.
    objects: HashMap<ID, Object>,
    /// Back references: value is all references pointing to the key.
    backlinks: HashMap<ID, HashSet<(Backkey, ID)>>,
    /// All claim objects, whether they are valid for permanode or not.
    claims: HashMap<ID, HashSet<ID>>,
    /// All permanodes, with valid associated claims.
    permanodes: HashMap<ID, Permanode>,
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
            claims: HashMap::new(),
            permanodes: HashMap::new(),
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

                index.insert_object_in_index(object);
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
    /// Insert the object, indexing the back references, and parsing the object
    /// to handle permanodes.
    fn insert_object_in_index(&mut self, object: Object) {
        assert!(!self.objects.contains_key(&object.id));
        {
            // Record reverse references
            // This is run on all values of type reference on the object,
            // whether it is a list or a dict
            let mut insert = |target: &ID, key: Backkey, source: ID| {
                if log_enabled!(LogLevel::Debug) {
                    match key {
                        Backkey::Key(ref k) => {
                            debug!("Reference {} -> {} ({})",
                                   source, target, k);
                        }
                        Backkey::Index(i) => {
                            debug!("Reference {} -> {} ({})",
                                   source, target, i);
                        }
                    }
                }

                // Add backlink
                insert_into_multimap(&mut self.backlinks,
                                     target, (key, source));
            };

            // Go over the object, calling insert() above on all its values of
            // type reference
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

        // Check for special objects
        if let ObjectData::Dict(ref dict) = object.data {
            match dict.get("dhstore_kind") {
                Some(&Property::String(ref kind)) => match kind as &str {
                    "permanode" => {
                        info!("Found permanode: {}", object.id);
                        self.index_permanode(&object);
                    }
                    "claim" => {
                        info!("Found claim: {}", object.id);
                        self.index_claim(&object);
                    }
                    kind => debug!("Found unknown kind {:?}", kind),
                },
                Some(_) => {
                    info!("Object has dhstore_kind with non-string value");
                }
                None => {}
            }
        }

        // Now inserts the object
        self.objects.insert(object.id.clone(), object);
    }

    fn index_permanode(&mut self, permanode: &Object) {
        // Validate the permanode
        let ref id = permanode.id;
        let permanode = match permanode.data {
            ObjectData::Dict(ref d) => d,
            ObjectData::List(_) => {
                panic!("Invalid permanode {}: not a dict", id);
            }
        };
        match permanode.get("random") {
            Some(&Property::String(ref s)) => {
                if s.len() != HASH_SIZE {
                    warn!("Invalid permanode {}: invalid random size {}",
                          id, s.len());
                    return;
                }
            }
            _ => {
                warn!("Invalid permanode {}: missing random", id);
                return;
            }
        }

        let sort = match permanode.get("sort") {
            Some(&Property::String(ref s)) => match s.parse() {
                Ok(f) => f,
                Err(()) => {
                    warn!("Invalid permanode {}: invalid sort", id);
                    return;
                }
            },
            _ => {
                warn!("Invalid permanode {}: invalid sort", id);
                return;
            }
        };

        let nodetype = match permanode.get("type") {
            Some(&Property::String(ref s)) => match s as &str {
                "set" | "single" => PermanodeType::Set,
                _ => {
                    warn!("Unknown permanode type {:?}, ignoring permanode {}",
                          s, id);
                    return;
                }
            },
            None => PermanodeType::Single,
            Some(_) => {
                warn!("Invalid permanode {}: invalid type", id);
                return;
            }
        };

        debug!("Permanode is well-formed, adding to index");
        let mut node = Permanode { sort: sort,
                                   nodetype: nodetype,
                                   claims: BTreeMap::new() };

        // Process claims
        if let Some(set) = self.claims.get(id) {
            for claim_id in set {
                let claim = self.objects.get(claim_id).unwrap();
                let claim = match claim.data {
                    ObjectData::Dict(ref d) => d,
                    _ => panic!("Invalid claim {}: not a dict", claim_id),
                };
                node.index_claim(claim, id, claim_id);
            }
        }

        // Insert the permanode in the index
        self.permanodes.insert(id.clone(), node);
    }

    fn index_claim(&mut self, claim: &Object) {
        // Validate the claim
        let id = &claim.id;
        let claim = match claim.data {
            ObjectData::Dict(ref d) => d,
            _ => panic!("Invalid claim {}: not a dict", id),
        };
        let permanode = match (claim.get("node"), claim.get("value")) {
            (Some(&Property::Reference(ref r)),
             Some(&Property::Reference(_))) => r,
            _ => {
                warn!("Invalid claim {}: wrong content", id);
                return;
            }
        };

        // Insert the claim in the index
        // Note that this means it is well-formed, not that it is valid;
        // validity needs to be checked with the permanode
        debug!("Claim is well-formed, adding to index");
        insert_into_multimap(&mut self.claims, permanode, id.clone());

        // If we have the permanode, index a valid claim
        if let Some(node) = self.permanodes.get_mut(permanode) {
            node.index_claim(claim, permanode, id);
        }
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
            self.insert_object_in_index(object);
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
