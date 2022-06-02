use actix_web::{HttpResponse, Responder};

pub async fn new_token() -> impl Responder {
    HttpResponse::Ok().body("todo!(\"get\")")
}

pub async fn delete_token() -> impl Responder {
    HttpResponse::Ok().body("todo!(\"get\")")
}
