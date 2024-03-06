use leptos::*;

use crate::components::*;

#[component]
pub fn Home() -> impl IntoView {
    view! { <Modal level=ModalLevel::Error>Test</Modal> }
}
