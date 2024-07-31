use gloo::utils::format::JsValueSerdeExt;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::{components::*, config::ProvideConfig, pages::Pages};
// use tracing_subscriber::fmt::format::Pretty;
// use tracing_subscriber::prelude::*;
// use tracing_web::{performance_layer, MakeWebConsoleWriter};

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <ProvideConfig>
            <ProvideToast>
                <Router>
                    <Stylesheet id="leptos" href="/pkg/mdoj.css"/>
                    <Title text="MDOJ"/>

                    <div class="bg-slate-950 w-full min-h-screen flex flex-col text-text">
                        <Navbar/>
                        <Pages/>
                        <Footer/>
                    </div>
                </Router>
            </ProvideToast>
        </ProvideConfig>
    }
}
