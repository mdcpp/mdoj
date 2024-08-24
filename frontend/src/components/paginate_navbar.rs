use leptos::*;
use leptos_router::*;
use tailwind_fuse::*;

use crate::utils::*;

#[component]
/// There are 4 different case
/// ```text
/// let right_half = size / 2
/// let left_half  = size - right_half
///
/// 0  1  [right_half] [page] [left_half]   ...   [max]  // leftmost page  < [max-1]
/// 0 ... [right_half] [page] [left_half] [max-1] [max]  // rightmost page >    1
/// 0 ... [right_half] [page] [left_half]   ...   [max]  // combination of the above
/// [0..page]          [page]              [page..=max]  //      [max]     < [size+2]
/// ```
pub fn PaginateNavbar(
    #[prop(default = 1)] size: u32,
    #[prop(into)] max_page: Signal<u32>,
    #[prop(into)] page: ParamsMapKey<u32>,
) -> impl IntoView {
    let left_half = size / 2;
    let right_half = size - left_half;
    let page_index = use_query_map().use_key_with_default(page);

    view! {
        <nav class="grid grid-flow-col auto-cols-max gap-1 text-center">
            <PaginateNavbarButton i=0 page />
            <Show when=move || 1 <= max_page()>
                <Show
                    when=move || page_index() <= left_half + 2
                    fallback=|| view! { <PaginateNavbarHidden /> }
                >
                    <PaginateNavbarButton i=1 page />
                </Show>
            </Show>
            <For
                each=move || {
                    page_index()
                        .saturating_sub(left_half)
                        .max(2)..=(page_index() + right_half).min(max_page().saturating_sub(2))
                }
                key=|i| *i
                children=move |i| {
                    view! { <PaginateNavbarButton i page /> }
                }
            ></For>
            <Show when=move || 2 <= max_page()>
                <Show
                    when=move || max_page() <= page_index() + right_half + 2
                    fallback=|| view! { <PaginateNavbarHidden /> }
                >
                    <PaginateNavbarButton
                        i=(move || max_page().saturating_sub(1)).into_signal()
                        page
                    />
                </Show>
            </Show>
            <Show when=move || 3 <= max_page()>
                <PaginateNavbarButton i=max_page page />
            </Show>
        </nav>
    }
}

#[component]
fn PaginateNavbarButton(
    #[prop(into)] i: MaybeSignal<u32>,
    #[prop(into)] page: ParamsMapKey<u32>,
) -> impl IntoView {
    let query_map = use_query_map();
    let href = query_map.with_key_map(move |map| map.set_key(page, Some(i())));
    let page = query_map.use_key_with_default(page);
    let disabled = create_memo(move |_| page() == i());
    view! {
        <A
            href
            class=move || {
                tw_join!(
                    "size-8",
                    if disabled() { "bg-primary disabled" } else { "bg-black-900" }
                )
            }
        >

            {i}
        </A>
    }
}

#[component]
fn PaginateNavbarHidden() -> impl IntoView {
    view! { <p class="bg-black-900 size-8">...</p> }
}
