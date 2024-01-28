use super::Merge;
use leptos::*;

#[component]
pub fn TextInput(
    #[prop(into, default = "text".to_owned().into())] kind: MaybeProp<String>,
    #[prop(into)] get: MaybeSignal<String>,
    #[prop(into, optional)] set: Option<WriteSignal<String>>,
    #[prop(into, optional)] placeholder: Option<AttributeValue>,
    #[prop(into, optional)] id: Option<AttributeValue>,
    #[prop(into, optional)] class: Option<AttributeValue>,
) -> impl IntoView {
    view! {
        <input
            class=Merge(
                class,
                "text-text outline-none p-2 bg-background border-2 rounded-md border-background focus:border-primary transition-colors",
            )
            id=id
            type=kind
            prop:value=get
            placeholder=placeholder
            on:input=move |e| {
                if let Some(set) = set {
                    set.set(event_target_value(&e));
                }
            }
        />
    }
}
