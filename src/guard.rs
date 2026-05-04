use std::collections::BTreeMap;
use uuid::Uuid;

use crate::store::Store;
use crate::predicate::Predicate;

#[derive(Debug)]
pub enum Denied {
    NoSession,
    NoActor,
    Forbidden,
}

impl std::fmt::Display for Denied {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Denied::NoSession => write!(f, "no session"),
            Denied::NoActor => write!(f, "no actor"),
            Denied::Forbidden => write!(f, "forbidden"),
        }
    }
}

impl actix_web::ResponseError for Denied {
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::FORBIDDEN
    }
}

pub async fn check(
    store: &dyn Store,
    header: Option<&str>,
    action: &str,
    resource: Option<Uuid>,
) -> Result<Option<Uuid>, Denied> {
    let session_id = match header {
        Some(val) if val.starts_with("Bearer ") => {
            val[7..].parse::<Uuid>().map_err(|_| Denied::NoSession)?
        }
        _ => return Ok(None),
    };

    let session = store.read(session_id).await.map_err(|_| Denied::NoSession)?
        .ok_or(Denied::NoSession)?;

    let actor = session.tags.get("actor")
        .and_then(|v| v.parse::<Uuid>().ok())
        .ok_or(Denied::NoActor)?;

    if resource.is_none() {
        return Ok(Some(actor));
    }

    let resource_id = resource.unwrap();
    let mut pred = Predicate::new();
    pred.insert("kind".into(), "perm".into());
    pred.insert("who".into(), actor.to_string());
    pred.insert("what".into(), resource_id.to_string());
    pred.insert("can".into(), action.to_string());

    let perms = store.query(&pred).await.map_err(|_| Denied::Forbidden)?;
    if !perms.is_empty() {
        return Ok(Some(actor));
    }

    let mut any_pred = pred.clone();
    any_pred.insert("can".into(), "*".into());
    let any_perms = store.query(&any_pred).await.map_err(|_| Denied::Forbidden)?;
    if !any_perms.is_empty() {
        return Ok(Some(actor));
    }

    Err(Denied::Forbidden)
}