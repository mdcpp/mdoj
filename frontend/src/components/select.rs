use leptos::*;
use tailwind_fuse::tw_join;

#[derive(Debug, Clone, PartialEq, Eq)]
struct SelectedValue(ReadSignal<String>);

#[component]
pub fn Select(
    children: Children,
    #[prop(into)] value: RwSignal<String>,
    #[prop(into, optional)] placeholder: Option<String>,
    #[prop(into, optional)] id: Option<AttributeValue>,
    #[prop(into, default = "".into())] class: String,
) -> impl IntoView {
    let (get, set) = value.split();
    provide_context(SelectedValue(get));
    view! {
        <select
            class=tw_join!(class, "text-text bg-black-800 p-2")
            id=id
            on:change=move |e| set(event_target_value(&e))
        >
            <option selected disabled hidden>
                {placeholder}
            </option>
            {children()}
        </select>
    }
}

#[component]
pub fn SelectOption(
    children: Children,
    #[prop(into)] value: String,
) -> impl IntoView {
    let selected_value = expect_context::<SelectedValue>().0;
    view! {
        <option value=value.clone() selected=move || selected_value() == value>
            {children()}
        </option>
    }
}
