use std::usize;

use leptos::*;
use tailwind_fuse::tw_join;

#[derive(Debug, Clone, PartialEq, Eq)]
struct SelectedValue(ReadSignal<usize>);

#[component]
pub fn Select<T>(
    options: Vec<(T, View)>,
    #[prop(into)] value: SignalSetter<T>,
    #[prop(into, optional)] placeholder: Option<View>,
    #[prop(into, optional)] id: Option<AttributeValue>,
    #[prop(into, default = "".into())] class: String,
) -> impl IntoView
where
    T: Clone + 'static,
{
    let (get, set) = create_signal(usize::MAX);
    provide_context(SelectedValue(get));

    let (children, map): (Vec<_>, Vec<_>) = options
        .into_iter()
        .enumerate()
        .map(|(value, (t, children))| {
            (view! { <SelectOption value>{children}</SelectOption> }, t)
        })
        .unzip();

    create_effect(move |_| {
        let i = get();
        if i == usize::MAX {
            return;
        }
        value(map[i].clone());
    });

    view! {
        <select
            class=tw_join!(class, "text-text text-center bg-black-800 p-2")
            id=id
            on:change=move |e| set(event_target_value(&e).parse().unwrap())
        >
            <option selected disabled hidden>
                {placeholder}
            </option>
            {children}
        </select>
    }
}

#[component]
fn SelectOption(children: Children, value: usize) -> impl IntoView {
    let selected_value = expect_context::<SelectedValue>().0;

    view! {
        <option value=value selected=move || selected_value() == value>
            {children()}
        </option>
    }
}
