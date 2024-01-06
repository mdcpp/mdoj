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
            class="text-text outline-none p-2 bg-background border-2 rounded-md border-background focus:border-primary transition-colors"
            class=class
            id=id
            type=kind
            prop:value=move || get.get()
            placeholder=placeholder
            on:change=move |e| {
                if let Some(set) = set {
                    set.set(event_target_value(&e));
                }
            }

            on:keyup=move |e| {
                if let Some(set) = set {
                    set.set(event_target_value(&e));
                }
            }
        />
    }
}
