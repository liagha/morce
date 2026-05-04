use dashmap::DashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::entity::Entity;
use crate::predicate::Predicate;

struct Subscription {
    predicate: Predicate,
    sender: mpsc::UnboundedSender<Entity>,
}

pub struct Hub {
    subs: DashMap<Uuid, Subscription>,
}

impl Hub {
    pub fn new() -> Self {
        Self {
            subs: DashMap::new(),
        }
    }

    pub fn subscribe(&self, predicate: Predicate) -> (Uuid, mpsc::UnboundedReceiver<Entity>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let id = Uuid::new_v4();
        self.subs.insert(id, Subscription {
            predicate,
            sender: tx,
        });
        (id, rx)
    }

    pub fn unsubscribe(&self, id: Uuid) {
        self.subs.remove(&id);
    }

    pub fn publish(&self, entity: &Entity) {
        if let Some(json) = entity.json() {
            let dead: Vec<Uuid> = self.subs
                .iter()
                .filter_map(|entry| {
                    let sub = entry.value();
                    if matches_predicate(&json, &sub.predicate) {
                        if sub.sender.send(entity.clone()).is_err() {
                            return Some(*entry.key());
                        }
                    }
                    None
                })
                .collect();
            for id in dead {
                self.subs.remove(&id);
            }
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