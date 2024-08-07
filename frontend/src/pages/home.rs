use leptos::*;

use crate::components::*;

#[component]
pub fn Home() -> impl IntoView {
    // <Modal level=ModalLevel::Error>Test</Modal>

    let toast = use_toast();
    view! {
        <Button on:click=move |_| toast(
            ToastVariant::Error,
            view! { "This is a error message............." }.into_view(),
        )>Click Me</Button>
        <Footer/>
    }
}
