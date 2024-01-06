use super::Caller;
use leptos::{ev::MouseEvent, *};

#[component]
pub fn Button(
    #[prop(into, default = "button".to_owned().into())] kind: MaybeProp<String>,
    #[prop(into, optional)] mut func: Caller<MouseEvent>,
    #[prop(into, optional)] id: Option<AttributeValue>,
    #[prop(into, optional)] class: Option<AttributeValue>,
    children: Children,
) -> impl IntoView {
    view! {
        <button
            class="text-background bg-primary p-2 rounded-md"
            type=kind
            id=id
            class=class
            on:click=move |e| {
                e.stop_propagation();
                func.call(e);
            }
        >

            {children()}
        </button>
    }
}
