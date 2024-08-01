use leptos::*;
use tailwind_fuse::tw_merge;

#[derive(Debug, Default, Clone, Copy)]
pub enum InputVariant {
    #[default]
    Text,
    Password,
    Textarea,
}

#[component]
pub fn Input(
    #[prop(into)] value: RwSignal<String>,
    #[prop(into, optional)] variant: InputVariant,
    #[prop(into, default = "".into())] class: String,
    #[prop(attrs)] attrs: Vec<(&'static str, Attribute)>,
) -> impl IntoView {
    let (get, set) = value.split();
    match variant {
        InputVariant::Text => view! {
            <input
                class=tw_merge!(
                    class,
                    "text-text outline-none p-2 bg-slate-800 border-b-2 border-slate-800 focus:border-primary transition-colors duration-300",
                )

                type="text"
                prop:value=get
                on:input=move |e| set(event_target_value(&e))
                {..attrs}
            />
        }.into_view(),
        InputVariant::Password => view! {
            <input
                class=tw_merge!(
                    class,
                    "text-text outline-none p-2 bg-slate-800 border-b-2 border-slate-800 focus:border-primary transition-colors duration-300",
                )

                type="password"
                prop:value=get
                on:input=move |e| set(event_target_value(&e))
                {..attrs}
            />
        }.into_view(),
        InputVariant::Textarea => view! {
            <textarea
                class=tw_merge!(
                    class,
                    "text-text outline-none p-2 bg-slate-800 border-b-2 border-slate-800 focus:border-primary transition-colors duration-300",
                )

                prop:value=get
                on:input=move |e| set(event_target_value(&e))
                {..attrs}
            ></textarea>
        }.into_view(),
    }
}
