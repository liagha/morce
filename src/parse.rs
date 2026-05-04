use std::collections::BTreeMap;

pub fn tags(raw: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for part in raw.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(eq) = part.find('=') {
            let key = part[..eq].to_string();
            let val = part[eq+1..].to_string();
            map.insert(key, val);
        }
    }
    map
}

pub fn predicate(raw: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for part in raw.split('&') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(eq) = part.find('=') {
            let key = part[..eq].to_string();
            let val = part[eq+1..].to_string();
            map.insert(key, val);
        }
    }
    map
}