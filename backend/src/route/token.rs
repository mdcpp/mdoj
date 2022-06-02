use actix_web::{HttpResponse, Responder};
// use openssl;

pub async fn new_token() -> impl Responder {
    // openssl::aes::AesKey::from(_)
    HttpResponse::Ok().body("todo!(\"get\")")
}

pub async fn delete_token() -> impl Responder {
    HttpResponse::Ok().body("todo!(\"get\")")
}
