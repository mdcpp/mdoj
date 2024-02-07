use crate::pages::*;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

// use tracing_subscriber::fmt::format::Pretty;
// use tracing_subscriber::prelude::*;
// use tracing_web::{performance_layer, MakeWebConsoleWriter};

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    // let fmt_layer = tracing_subscriber::fmt::layer()
    //     .with_ansi(false) // Only partially supported across browsers
    //     .without_time() // std::time is not available in browsers, see note below
    //     .with_writer(MakeWebConsoleWriter::new()); // write events to the console
    // let perf_layer = performance_layer().with_details_from_fields(Pretty::default());
    // tracing_subscriber::registry()
    //     .with(fmt_layer)
    //     .with(perf_layer)
    //     .init(); // Install these as subscribers to tracing events
    // tracing::

    view! {
        <Router>
            <Stylesheet id="leptos" href="/pkg/mdoj.css"/>
            <Title text="MDOJ"/>

            <Main/>
        </Router>
    }
}
