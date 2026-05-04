use bytes::Bytes;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Entity {
    pub id: Uuid,
    pub load: Bytes,
}

impl Entity {
    pub fn new(load: Bytes) -> Self {
        Self {
            id: Uuid::new_v4(),
            load,
        }
    }

    pub fn json(&self) -> Option<serde_json::Value> {
        serde_json::from_slice(&self.load).ok()
    }
}

impl Serialize for Entity {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Entity", 2)?;
        state.serialize_field("id", &self.id)?;
        let value = self.json().unwrap_or(serde_json::Value::Null);
        state.serialize_field("load", &value)?;
        state.end()
    }
}