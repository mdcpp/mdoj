use leptos::*;
use leptos_icons::*;

#[component]
pub fn Unimplemented() -> impl IntoView {
    view! {
        <main class="grow size-full flex flex-col justify-evenly items-center">
            <h1 class="text-2xl text-text">
                "小心鷹架！這個頁面還在施工中"
            </h1>
            <Icon icon=icondata::FaPeopleCarryBoxSolid class="size-40" />
        </main>
    }
}
