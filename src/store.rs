use async_trait::async_trait;
use uuid::Uuid;

use crate::entity::Entity;
use crate::predicate::Predicate;

#[derive(Debug)]
pub enum Error {
    NotFound,
    Internal(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotFound => write!(f, "not found"),
            Error::Internal(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl actix_web::ResponseError for Error {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Error::NotFound => actix_web::http::StatusCode::NOT_FOUND,
            Error::Internal(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[async_trait]
pub trait Store: Send + Sync {
    async fn create(&self, load: bytes::Bytes) -> Result<Entity, Error>;
    async fn read(&self, id: Uuid) -> Result<Option<Entity>, Error>;
    async fn update(&self, id: Uuid, load: bytes::Bytes) -> Result<Entity, Error>;
    async fn delete(&self, id: Uuid) -> Result<(), Error>;
    async fn query(&self, predicate: &Predicate) -> Result<Vec<Entity>, Error>;
}