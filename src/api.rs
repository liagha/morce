use actix_web::{web, HttpRequest, HttpResponse};
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::hub::Hub;
use crate::memory::Memory;
use crate::predicate::Predicate;
use crate::store::Store;
use crate::format;
use crate::parse;

pub struct State {
    pub store: std::sync::Arc<Memory>,
    pub hub: std::sync::Arc<Hub>,
}

fn extract_tags(req: &HttpRequest) -> BTreeMap<String, String> {
    if let Some(header) = req.headers().get("x-tags") {
        if let Ok(val) = header.to_str() {
            return parse::tags(val);
        }
    }
    BTreeMap::new()
}

fn tags_header(tags: &BTreeMap<String, String>) -> String {
    let mut parts = Vec::new();
    for (k, v) in tags {
        parts.push(format!("{}={}", k, v));
    }
    parts.join(",")
}

pub async fn create(
    state: web::Data<State>,
    req: HttpRequest,
    body: web::Bytes,
) -> actix_web::Result<HttpResponse> {
    let tags = extract_tags(&req);
    let entity = state.store.create(body, tags).await?;
    state.hub.publish(&entity);
    Ok(HttpResponse::Created()
        .insert_header(("x-entity-id", entity.id.to_string()))
        .insert_header(("x-entity-tags", tags_header(&entity.tags)))
        .body(entity.load.clone()))
}

pub async fn read(
    state: web::Data<State>,
    path: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    let id = path.into_inner();
    if let Some(entity) = state.store.read(id).await? {
        Ok(HttpResponse::Ok()
            .insert_header(("x-entity-id", entity.id.to_string()))
            .insert_header(("x-entity-tags", tags_header(&entity.tags)))
            .body(entity.load))
    } else {
        Err(actix_web::error::ErrorNotFound("not found"))
    }
}

pub async fn update(
    state: web::Data<State>,
    path: web::Path<Uuid>,
    req: HttpRequest,
    body: web::Bytes,
) -> actix_web::Result<HttpResponse> {
    let id = path.into_inner();
    let tags = extract_tags(&req);
    let entity = state.store.update(id, body, tags).await?;
    state.hub.publish(&entity);
    Ok(HttpResponse::Ok()
        .insert_header(("x-entity-id", entity.id.to_string()))
        .insert_header(("x-entity-tags", tags_header(&entity.tags)))
        .body(entity.load))
}

pub async fn delete(
    state: web::Data<State>,
    path: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    let id = path.into_inner();
    state.store.delete(id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn query(
    state: web::Data<State>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let mut predicate = Predicate::new();
    for (key, val) in req.query_string().split('&').filter_map(|part| {
        let mut split = part.splitn(2, '=');
        Some((split.next()?.to_string(), split.next()?.to_string()))
    }) {
        predicate.insert(key, val);
    }
    let entities = state.store.query(&predicate).await?;
    Ok(HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(format::entity_list(&entities)))
}