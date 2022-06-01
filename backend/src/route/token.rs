use actix_web::{route, HttpResponse, Responder};

pub fn new_token() -> impl Responder {
    HttpResponse::Ok().body("todo!(\"get\")")
}

pub fn delete_token() -> impl Responder {
    HttpResponse::Ok().body("todo!(\"get\")")
}
