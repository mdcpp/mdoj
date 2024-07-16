use gloo::utils::format::JsValueSerdeExt;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::{config::ProvideConfig, pages::*};

// use tracing_subscriber::fmt::format::Pretty;
// use tracing_subscriber::prelude::*;
// use tracing_web::{performance_layer, MakeWebConsoleWriter};

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Script src="https://cdn.jsdelivr.net/npm/monaco-editor@0.50.0/min/vs/loader.js" defer=""/>
        <ProvideConfig>
            <Router>
                <Stylesheet id="leptos" href="/pkg/mdoj.css"/>
                <Title text="MDOJ"/>

                <Main/>
            </Router>
        </ProvideConfig>
    }
}
