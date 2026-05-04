use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::entity::Entity;

pub fn entity(e: &Entity) -> String {
    let mut out = format!("id:{}\ntags:", e.id);
    for (i, (k, v)) in e.tags.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!("{}={}", k, v));
    }
    out.push('\n');
    out.push_str("load:");
    let load_display = std::str::from_utf8(&e.load)
        .map(|s| s.to_string())
        .unwrap_or_else(|_| STANDARD.encode(&e.load));
    out.push_str(&load_display);
    out
}

pub fn entity_list(list: &[Entity]) -> String {
    let mut out = String::new();
    for e in list {
        out.push_str(&entity(e));
        out.push('\n');
    }
    out
}