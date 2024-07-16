use leptos::*;
use tailwind_fuse::tw_merge;

#[component]
pub fn TextInput(
    #[prop(into, default = "text".to_owned().into())] kind: MaybeProp<String>,
    #[prop(into)] value: RwSignal<String>,
    #[prop(into, optional)] placeholder: Option<AttributeValue>,
    #[prop(into, optional)] id: Option<AttributeValue>,
    #[prop(into, default = "".into())] class: String,
) -> impl IntoView {
    let (get, set) = value.split();
    view! {
        <input
            class=tw_merge!(
                class,
                "text-text outline-none p-2 bg-slate-800 border-b-2 border-slate-800 focus:border-primary transition-colors duration-300",
            )

            id=id
            type=kind
            prop:value=get
            placeholder=placeholder
            on:input=move |e| set(event_target_value(&e))
        />
    }
}
