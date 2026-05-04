use uuid::Uuid;
use crate::store::Store;

pub async fn resolve_bearer(token: &str, store: &dyn Store) -> Option<Uuid> {
    let id: Uuid = token.parse().ok()?;
    let entity = store.read(id).await.ok()??;
    let json = entity.json()?;
    match json.get("type")? {
        serde_json::Value::String(s) if s == "session" => {
            json.get("user")?.as_str()?.parse().ok()
        }
        _ => None,
    }
}