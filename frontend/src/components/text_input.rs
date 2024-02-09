use leptos::*;

use super::Merge;

#[component]
pub fn TextInput(
    #[prop(into, default = "text".to_owned().into())] kind: MaybeProp<String>,
    #[prop(into)] value: RwSignal<String>,
    #[prop(into, optional)] placeholder: Option<AttributeValue>,
    #[prop(into, optional)] id: Option<AttributeValue>,
    #[prop(into, optional)] class: Option<AttributeValue>,
) -> impl IntoView {
    let (get, set) = value.split();
    view! {
        <input
            class=Merge(
                class,
                "text-text outline-none p-2 bg-background border-2 rounded-md border-background focus:border-primary transition-colors duration-300",
            )

            id=id
            type=kind
            prop:value=get
            placeholder=placeholder
            on:input=move |e| set(event_target_value(&e))
        />
    }
}
