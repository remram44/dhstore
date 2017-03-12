use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::path::{PathBuf, Path};

use common::{ID, Object, ObjectData, ObjectIndex, Property};
use errors;
use serialize;

pub enum PolicyDecision {
    Get,
    Keep,
    Drop,
}

pub trait Policy {
    fn handle(&mut self, property: &str, object: Object)
              -> (PolicyDecision, Box<Policy>);
}

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

pub enum RefCount {
    Number(usize),
    Special,
}

pub struct RefCountedObject {
    refs: RefCount,
    object: Object,
}

#[derive(PartialEq, Eq, Hash)]
enum Backkey {
    Index(usize),
    Key(String),
}

pub struct MemoryIndex {
    objects: HashMap<ID, RefCountedObject>,
    backlinks: HashMap<ID, HashSet<(Backkey, ID)>>,
    root: ID,
    policy: Box<Policy>,
}

impl MemoryIndex {
    pub fn open<P: AsRef<Path>>(path: P, root: ID)
        -> errors::Result<MemoryIndex>
    {
        let mut objects = HashMap::new();
        let mut backlinks: HashMap<ID, HashSet<(Backkey, ID)>> = HashMap::new();
        let dirlist = path.as_ref().read_dir()
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

                // Record reverse references
                let mut insert = |target: &ID, key: Backkey, source: ID| {
                    if let Some(set) = backlinks.get_mut(target) {
                        set.insert((key, source));
                        return;
                    }
                    let mut set = HashSet::new();
                    set.insert((key, source));
                    backlinks.insert(target.clone(), set);
                };
                match object.data {
                    ObjectData::Dict(ref dict) => for (k, v) in dict {
                        match v {
                            &Property::Reference(ref id) => {
                                insert(id,
                                       Backkey::Key(k.clone()),
                                       object.id.clone());
                            }
                            _ => {}
                        }
                    },
                    ObjectData::List(ref list) => for (k, v) in list.into_iter().enumerate() {
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

                objects.insert(object.id.clone(),
                               RefCountedObject { refs: RefCount::Number(0),
                                                  object: object });
            }
        }

        // TODO: Parse root config

        Ok(MemoryIndex {
            objects: objects,
            backlinks: backlinks,
            root: root,
            policy: Box::new(PolicyV1::new()),
        })
    }
}

impl ObjectIndex for MemoryIndex {
    fn add(&mut self, data: ObjectData) -> errors::Result<ID> {
        let object = serialize::hash_object(data);
        // TODO: Add to index
        unimplemented!()
    }

    fn verify(&mut self, collect: bool) -> errors::Result<()> {
        let mut alive = HashSet::new(); // ids
        let mut open = VecDeque::new(); // objects
        match self.objects.get(&self.root) {
            None => error!("Root is missing: {}", self.root),
            Some(obj) => open.push_front(obj),
        }
        while let Some(object) = open.pop_back() {
            let id = &object.object.id;
            debug!("Walking, open={}, alive={}/{}, id={}",
                   open.len(), alive.len(), self.objects.len(), id);
            if !alive.contains(id) {
                alive.insert(id);
            }
        }
        Ok(())
    }
}
