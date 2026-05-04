use actix_web::{HttpRequest, HttpResponse};

pub async fn page(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("console.html"))
}