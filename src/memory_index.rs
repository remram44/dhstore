use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path};

use common::{ID, Object, ObjectIndex};

pub enum PolicyDecision {
    Get,
    Keep,
    Drop,
}

pub trait Policy {
    fn handle(&mut self, property: &str, object: Object
    ) -> (PolicyDecision, Policy);
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
    roots: HashSet<ID>,
    policy: Box<Policy>,
}

impl MemoryIndex {
    pub fn collect_garbage(&mut self) {
        let mut alive = HashSet::new(); // ids
        let mut open = self.roots.iter() // objects
            .filter_map(|id| {
                let obj = self.objects.get(id);
                if obj.is_none() {
                    warn!("Root is missing: {}", id);
                }
                obj
            })
            .collect::<VecDeque<_>>();
        while let Some(object) = open.pop_back() {
            let id = &object.object.id;
            if !alive.contains(id) {
                alive.insert(id);
            }
        }
    }

    pub fn open<P: AsRef<Path>>(path: P) -> MemoryIndex {
        unimplemented!()
    }
}

impl ObjectIndex for MemoryIndex {
}
