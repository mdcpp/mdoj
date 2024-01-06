use leptos::*;
use leptos_router::*;

#[component]
pub fn RedirectIf<P: AsRef<str> + 'static>(
    #[prop(into)] condition: MaybeSignal<bool>,
    path: P,
) -> impl IntoView {
    let navigate = use_navigate();
    create_effect(move |_| {
        if condition() {
            navigate(path.as_ref(), Default::default());
        }
    });
}
