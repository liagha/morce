use dashmap::DashMap;
use dashmap::DashSet;
use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::predicate::Predicate;

pub struct Index {
    keys: DashMap<String, DashMap<String, Arc<DashSet<Uuid>>>>,
}

impl Index {
    pub fn new() -> Self {
        Self {
            keys: DashMap::new(),
        }
    }

    pub fn insert(&self, id: Uuid, tags: &BTreeMap<String, String>) {
        for (key, val) in tags {
            let column = self.keys.entry(key.clone()).or_insert_with(DashMap::new);
            let set = column.entry(val.clone()).or_insert_with(|| Arc::new(DashSet::new()));
            set.insert(id);
        }
    }

    pub fn remove(&self, id: Uuid, tags: &BTreeMap<String, String>) {
        for (key, val) in tags {
            if let Some(column) = self.keys.get(key) {
                if let Some(set) = column.get(val) {
                    set.remove(&id);
                }
            }
        }
    }

    pub fn find(&self, predicate: &Predicate) -> Option<Vec<Uuid>> {
        let mut sets: Vec<Arc<DashSet<Uuid>>> = Vec::new();
        for (key, val) in predicate {
            if let Some(column) = self.keys.get(key) {
                if let Some(set) = column.get(val) {
                    sets.push(Arc::clone(&*set));
                } else {
                    return Some(Vec::new());
                }
            } else {
                return None;
            }
        }
        if sets.is_empty() {
            return None;
        }
        if sets.len() == 1 {
            return Some(sets[0].iter().map(|id| *id).collect());
        }
        let smallest = sets.iter().min_by_key(|s| s.len()).unwrap();
        let result: Vec<Uuid> = smallest
            .iter()
            .filter(|id| sets.iter().all(|s| s.contains(id)))
            .map(|id| *id)
            .collect();
        Some(result)
    }

    pub fn is_indexed(&self, key: &str) -> bool {
        self.keys.contains_key(key)
    }
}