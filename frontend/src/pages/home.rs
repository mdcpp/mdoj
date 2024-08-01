use leptos::*;

use crate::components::*;

#[component]
pub fn Home() -> impl IntoView {
    // <Modal level=ModalLevel::Error>Test</Modal>

    view! {
        <Button on:click=|_| toast(
            view! { "This is a error message............." },
        )>Click Me</Button>
    }
}
