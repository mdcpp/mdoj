use leptos::{ev::MouseEvent, *};

use super::Merge;

#[component]
pub fn Button(
    #[prop(into, default = "button".to_owned().into())] kind: MaybeSignal<
        String,
    >,
    #[prop(into, default = false.into())] disabled: MaybeSignal<bool>,
    #[prop(into, optional)] on_click: Option<Callback<MouseEvent>>,
    #[prop(into, optional)] id: Option<AttributeValue>,
    #[prop(into, optional)] class: Option<AttributeValue>,
    children: Children,
) -> impl IntoView {
    view! {
        <button
            class=Merge(
                class,
                "text-background bg-primary p-2 rounded-md disabled:cursor-not-allowed disabled:brightness-50",
            )

            type=kind
            disabled=disabled
            id=id
            on:click=move |e| {
                if let Some(f) = on_click {
                    e.stop_propagation();
                    f(e);
                }
            }
        >

            {children()}
        </button>
    }
}
