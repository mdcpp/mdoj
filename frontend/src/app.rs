use leptos::*;
use leptos_meta::*;
use leptos_query_devtools::LeptosQueryDevtools;
use leptos_router::*;

use crate::{
    components::*,
    pages::Pages,
    utils::{config::ProvideConfig, *},
};
// use tracing_subscriber::fmt::format::Pretty;
// use tracing_subscriber::prelude::*;
// use tracing_web::{performance_layer, MakeWebConsoleWriter};

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    provide_query_service();

    view! {
        <Stylesheet id="leptos" href="/pkg/mdoj.css" />
        <Title text="MDOJ" />
        <Link rel="icon" type_="image/svg+xml" href="/assets/favicon.svg" />
        <Link rel="icon" type_="image/png" href="/assets/favicon.png" />
        <ProvideConfig>
            <ProvideToast>
                <div class="bg-black-950 w-full min-h-dvh flex flex-col text-text">
                    <Router>
                        <Pages />
                    </Router>
                </div>
            </ProvideToast>
        </ProvideConfig>
        <LeptosQueryDevtools />
    }
}
