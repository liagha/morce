use actix_web::{web, HttpRequest, HttpResponse};
use bytes::Bytes;
use uuid::Uuid;

use crate::hub::Hub;
use crate::memory::Memory;
use crate::predicate::Predicate;
use crate::store::Store;

pub struct State {
    pub store: std::sync::Arc<Memory>,
    pub hub: std::sync::Arc<Hub>,
}

#[derive(serde::Deserialize)]
pub struct CreatePayload {
    pub load: serde_json::Value,
}

pub async fn create(
    state: web::Data<State>,
    body: web::Json<CreatePayload>,
) -> actix_web::Result<HttpResponse> {
    let bytes: Bytes = serde_json::to_vec(&body.load)
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?
        .into();
    let entity = state.store.create(bytes).await?;
    state.hub.publish(&entity);
    Ok(HttpResponse::Ok().json(&entity))
}

pub async fn read(
    state: web::Data<State>,
    path: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    let id = path.into_inner();
    if let Some(entity) = state.store.read(id).await? {
        Ok(HttpResponse::Ok().json(&entity))
    } else {
        Err(actix_web::error::ErrorNotFound("not found"))
    }
}

pub async fn update(
    state: web::Data<State>,
    path: web::Path<Uuid>,
    body: web::Json<CreatePayload>,
) -> actix_web::Result<HttpResponse> {
    let id = path.into_inner();
    let bytes: Bytes = serde_json::to_vec(&body.load)
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?
        .into();
    let entity = state.store.update(id, bytes).await?;
    state.hub.publish(&entity);
    Ok(HttpResponse::Ok().json(&entity))
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
        if let Ok(parsed) = serde_json::from_str(&val) {
            predicate.insert(key, parsed);
        } else {
            predicate.insert(key, serde_json::Value::String(val));
        }
    }
    let entities = state.store.query(&predicate).await?;
    Ok(HttpResponse::Ok().json(&entities))
}