use leptos::*;
use num_traits::PrimInt;
use tailwind_fuse::tw_join;
use wasm_bindgen::JsValue;

#[component]
pub fn InputNumber<T>(
    value: RwSignal<T>,
    #[prop(into, default = "".into())] class: String,
    #[prop(attrs)] attrs: Vec<(&'static str, Attribute)>,
) -> impl IntoView
where
    T: PrimInt + 'static,
    JsValue: From<T>,
{
    let (get, set) = value.split();
    let on_input = move |e| {
        let value =
            T::from_str_radix(&event_target_value(&e), 10).unwrap_or(T::zero());
        set(value);
    };
    view! {
        <input
            class=tw_join!(
                class,
                "text-text outline-none p-2 bg-black-800 border-b-2 border-black-800 focus:border-primary transition-colors duration-300",
            )

            {..attrs}

            type="number"
            prop:value=get
            on:input=on_input
        />
    }
}
