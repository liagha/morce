use bytes::Bytes;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Entity {
    pub id: Uuid,
    pub load: Bytes,
    pub tags: BTreeMap<String, String>,
}

impl Entity {
    pub fn new(load: Bytes, tags: BTreeMap<String, String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            load,
            tags,
        }
    }
}