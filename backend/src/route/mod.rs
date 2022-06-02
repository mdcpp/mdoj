mod token;
use actix_web::web;

pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    // auth service
    cfg.service(
        web::resource("/token")
            .route(web::post().to(token::new_token))
            .route(web::delete().to(token::delete_token)),
    );
}
