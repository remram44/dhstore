use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use common::{ID, Object, ObjectIndex};
use errors;

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
        unimplemented!()
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

pub struct MemoryIndex {
    objects: HashMap<ID, RefCountedObject>,
    properties: HashMap<String, HashSet<RefCountedObject>>,
    root: ID,
    policy: Box<Policy>,
}

impl MemoryIndex {
    pub fn open<P: AsRef<Path>>(path: P, root: ID)
        -> errors::Result<MemoryIndex>
    {
        Ok(MemoryIndex {
            objects: HashMap::new(),
            properties: HashMap::new(),
            root: root,
            policy: Box::new(PolicyV1::new()),
        })
    }
}

impl ObjectIndex for MemoryIndex {
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
