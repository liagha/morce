mod entity;
mod predicate;
mod store;
mod index;
mod memory;
mod hub;
mod auth;
mod api;
mod ws;

use actix_web::{web, App, HttpServer};
use std::sync::Arc;

use api::State;
use memory::Memory;
use hub::Hub;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let store = Arc::new(Memory::new());
    let hub = Arc::new(Hub::new());

    let state = web::Data::new(State {
        store: store.clone(),
        hub: hub.clone(),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/entities", web::post().to(api::create))
            .route("/entities", web::get().to(api::query))
            .route("/entities/{id}", web::get().to(api::read))
            .route("/entities/{id}", web::put().to(api::update))
            .route("/entities/{id}", web::delete().to(api::delete))
            .route("/ws", web::get().to(ws::handler))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}