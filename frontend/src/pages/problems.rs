use leptos::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

use crate::{components::*, errors::*, session::use_token};
const PAGESIZE: u64 = 10;
#[component]
pub fn Problems() -> impl IntoView {
    view! {
        <div class="h-full container container-lg items-center justify-between text-lg">// <ErrorBoundary fallback=error_fallback>
        //
        // </ErrorBoundary>
        </div>
    }
}
