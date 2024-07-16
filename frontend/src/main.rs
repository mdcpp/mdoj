#[cfg(feature = "ssr")]
use anyhow::Result;

#[cfg(feature = "ssr")]
#[actix_web::main]
async fn main() -> Result<()> {
    use actix_files::Files;
    use actix_web::{dev::Service, http::header, *};
    // use frontend::{app::*, config};
    use frontend::{
        app::*,
        config::{backend_config, init_config},
    };
    use leptos::*;
    use leptos_actix::{generate_route_list, LeptosRoutes};

    init_config().await?;

    let conf = get_configuration(None).await?;
    let addr = conf.leptos_options.site_addr;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);
    println!("listening on http://{}", &addr);
    init_config().await?;

    HttpServer::new(move || {
        let leptos_options: &LeptosOptions = &conf.leptos_options;
        let site_root = &leptos_options.site_root;

        let app = App::new()
            .route("/api/{tail:.*}", leptos_actix::handle_server_fns())
            // serve JS/WASM/CSS from `pkg`
            .service(Files::new("/pkg", format!("{site_root}/pkg")))
            // serve other assets from the `assets` directory
            .service(Files::new("/assets", site_root))
            // serve the favicon from /favicon.ico
            .service(favicon)
            .leptos_routes(leptos_options.to_owned(), routes.to_owned(), App)
            .app_data(web::Data::new(leptos_options.to_owned()))
            .wrap_fn(|mut req, srv| {
                if !backend_config().trust_xff {
                    if let Some(addr) = req.peer_addr() {
                        let headers = req.headers_mut();
                        headers.insert(
                            header::X_FORWARDED_FOR,
                            addr.to_string()
                                .try_into()
                                .expect("should never panic"),
                        );
                    }
                }
                srv.call(req)
            });

        #[cfg(feature = "compress")]
        let app = app.wrap(middleware::Compress::default());

        app
    })
    .bind(&addr)?
    .run()
    .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
#[actix_web::get("favicon.ico")]
async fn favicon(
    leptos_options: actix_web::web::Data<leptos::LeptosOptions>,
) -> actix_web::Result<actix_files::NamedFile> {
    let leptos_options = leptos_options.into_inner();
    let site_root = &leptos_options.site_root;
    Ok(actix_files::NamedFile::open(format!(
        "{site_root}/favicon.ico"
    ))?)
}

#[cfg(not(any(feature = "ssr", feature = "csr")))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
    // see optional feature `csr` instead
}

#[cfg(all(not(feature = "ssr"), feature = "csr"))]
pub fn main() {
    // a client-side main function is required for using `trunk serve`
    // prefer using `cargo leptos serve` instead
    // to run: `trunk serve --open --features csr`
    use leptos::*;
    use mdoj::app::*;

    console_error_panic_hook::set_once();

    leptos::mount_to_body(App);
}
