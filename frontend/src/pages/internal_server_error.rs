use leptos::*;

/// 500 - Internal Server Error
#[component]
pub fn InternalServerError() -> impl IntoView {
    // set an HTTP status code 500
    // this is feature gated because it can only be done during
    // initial server-side rendering
    // if you navigate to the 500 page subsequently, the status
    // code will not be set because there is not a new HTTP request
    // to the server
    #[cfg(feature = "ssr")]
    {
        // this can be done inline because it's synchronous
        // if it were async, we'd use a server function
        let resp = expect_context::<leptos_actix::ResponseOptions>();
        resp.set_status(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
    }

    view! {
        <main class="grow flex items-center justify-center">
            <h1 class="text-9xl text-text">"Something is broken"</h1>
        </main>
    }
}
