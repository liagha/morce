use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws;
use futures_util::StreamExt;

use crate::api::State;
use crate::format;
use crate::parse;

pub async fn handler(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<State>,
) -> actix_web::Result<HttpResponse> {
    let (response, session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    let hub = state.hub.clone();
    actix_web::rt::spawn(async move {
        let mut sub_id = None;
        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                actix_ws::Message::Text(text) => {
                    let predicate = parse::predicate(&text);
                    if predicate.is_empty() {
                        continue;
                    }
                    if let Some(id) = sub_id {
                        hub.unsubscribe(id);
                    }
                    let (id, mut rx) = hub.subscribe(predicate);
                    sub_id = Some(id);
                    let mut sender = session.clone();
                    actix_web::rt::spawn(async move {
                        while let Some(entity) = rx.recv().await {
                            let text = format::entity(&entity);
                            let _ = sender.text(text).await;
                        }
                    });
                }
                actix_ws::Message::Close(_) => break,
                _ => {}
            }
        }
        if let Some(id) = sub_id {
            hub.unsubscribe(id);
        }
    });

    Ok(response)
}