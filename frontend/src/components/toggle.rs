use leptos::*;
use tailwind_fuse::*;

#[component]
pub fn Toggle(
    #[prop(into, default = "checkbox".to_owned().into())] kind: MaybeProp<
        String,
    >,
    #[prop(into)] value: RwSignal<bool>,
    #[prop(into, optional)] id: Option<AttributeValue>,
    #[prop(into, default = "".into())] class: String,
) -> impl IntoView {
    let (get, set) = value.split();
    view! {
        <label class=tw_join!(class, "inline-flex items-center cursor-pointer")>
            <input
                type=kind
                id=id
                prop:value=get
                class="sr-only peer"
                on:input=move |e| set(event_target_value(&e) == "true")
            />
            <div class="relative w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
        </label>
    }
}
