use super::Merge;
use leptos::{ev::MouseEvent, *};

#[component]
pub fn Button(
    #[prop(into, default = "button".to_owned().into())] kind: MaybeSignal<String>,
    #[prop(into, default = false.into())] disabled: MaybeSignal<bool>,
    #[prop(into, optional)] on_click: Option<Callback<MouseEvent>>,
    #[prop(into, optional)] id: Option<AttributeValue>,
    #[prop(into, optional)] class: Option<AttributeValue>,
    children: Children,
) -> impl IntoView {
    view! {
        <button
            class=Merge(class, "text-background bg-primary p-2 rounded-md")
            type=kind
            disabled=disabled
            id=id
            on:click=move |e| {
                on_click
                    .map(|f| {
                        e.stop_propagation();
                        f(e);
                    });
            }
        >

            {children()}
        </button>
    }
}
