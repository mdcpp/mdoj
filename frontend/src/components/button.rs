use leptos::*;
use tailwind_fuse::*;

#[derive(TwVariant)]
pub enum ButtonVariant {
    #[tw(default, class = "text-background bg-primary")]
    Primary,
    #[tw(class = "text-background bg-secondary")]
    Secondary,
    #[tw(class = "text-black-950 bg-accent")]
    Accent,
}

#[component]
pub fn Button(
    #[prop(into, default = "button".to_owned().into())] type_: String,
    #[prop(into, optional)] variant: ButtonVariant,
    #[prop(into, optional)] disabled: MaybeSignal<bool>,
    #[prop(into, optional)] id: Option<AttributeValue>,
    #[prop(into, default = "".into())] class: String,
    children: Children,
) -> impl IntoView {
    view! {
        <button
            class=tw_join!(
                class, variant,
                "p-2 hover:brightness-110 disabled:cursor-not-allowed disabled:brightness-50 transition-all",
            )

            type=type_
            disabled=disabled
            id=id
        >

            {children()}
        </button>
    }
}
