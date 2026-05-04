use async_trait::async_trait;
use dashmap::DashMap;
use uuid::Uuid;

use crate::entity::Entity;
use crate::index::Index;
use crate::predicate::Predicate;
use crate::store::{Error, Store};

pub struct Memory {
    items: DashMap<Uuid, Entity>,
    index: Index,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            items: DashMap::new(),
            index: Index::new(),
        }
    }

    fn update_index(&self, entity: &Entity) {
        if let Some(json) = entity.json() {
            self.index.insert(entity.id, &json);
        }
    }

    fn remove_index(&self, entity: &Entity) {
        if let Some(json) = entity.json() {
            self.index.remove(entity.id, &json);
        }
    }
}

#[async_trait]
impl Store for Memory {
    async fn create(&self, load: bytes::Bytes) -> Result<Entity, Error> {
        let entity = Entity::new(load);
        self.items.insert(entity.id, entity.clone());
        self.update_index(&entity);
        Ok(entity)
    }

    async fn read(&self, id: Uuid) -> Result<Option<Entity>, Error> {
        Ok(self.items.get(&id).map(|e| e.clone()))
    }

    async fn update(&self, id: Uuid, load: bytes::Bytes) -> Result<Entity, Error> {
        let mut entity = self.items.get_mut(&id).ok_or(Error::NotFound)?;
        self.remove_index(&entity);
        entity.load = load;
        self.update_index(&entity);
        Ok(entity.clone())
    }

    async fn delete(&self, id: Uuid) -> Result<(), Error> {
        if let Some((_, entity)) = self.items.remove(&id) {
            self.remove_index(&entity);
            Ok(())
        } else {
            Err(Error::NotFound)
        }
    }

    async fn query(&self, predicate: &Predicate) -> Result<Vec<Entity>, Error> {
        if let Some(ids) = self.index.find(predicate) {
            let mut result = Vec::new();
            for id in ids {
                if let Some(entity) = self.items.get(&id) {
                    result.push(entity.clone());
                }
            }
            Ok(result)
        } else {
            let mut result = Vec::new();
            for entry in self.items.iter() {
                let entity = entry.value();
                if let Some(json) = entity.json() {
                    if matches_predicate(&json, predicate) {
                        result.push(entity.clone());
                    }
                }
            }
            Ok(result)
        }
    }
}

fn matches_predicate(json: &serde_json::Value, predicate: &Predicate) -> bool {
    for (key, val) in predicate {
        match json.get(key) {
            Some(v) if v == val => continue,
            _ => return false,
        }
    }
    true
}