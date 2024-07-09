use leptos::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

use crate::{
    components::*,
    error::*,
    grpc::{problem_set_client::*, *},
    pages::{error_fallback, problems::toggle::Toggle},
    session::use_token,
};
const PAGESIZE: u64 = 10;
#[component]
pub fn Problems() -> impl IntoView {
    view! {
        <div class="h-full container container-lg items-center justify-between text-lg">
            // <ErrorBoundary fallback=error_fallback>
            //
            // </ErrorBoundary>
        </div>
    }
}
