use actix_web::{middleware, web::Data, App, HttpServer};
mod route;
mod state;
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let host = env::var("HOST").unwrap_or("0.0.0.0".to_owned());
    let port = env::var("PORT")
        .unwrap_or("8080".to_owned())
        .parse::<u16>()
        .unwrap();

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(Data::new(state::generate_state()))
            .configure(route::configure)
    })
    .bind((host, port))?
    .run()
    .await
}
