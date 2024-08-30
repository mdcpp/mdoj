use leptos::*;
use leptos_router::*;
use tailwind_fuse::*;

use crate::utils::*;

#[component]
pub fn PaginateTable<const N: usize, S, H>(
    #[prop(into)] headers: [(Option<S::Output>, View); N],
    #[prop(into)] rows: Vec<(H, [View; N])>,
    #[prop(into, optional)] class: String,
    #[prop(into)] sort: ParamsMapKey<S>,
    #[prop(into)] order: ParamsMapKey<GrpcEnum<grpc::Order>>,
) -> impl IntoView
where
    S: ParamsMapValue + 'static,
    H: ToHref + 'static,
{
    let query_map = use_query_map();
    let headers = headers.map(|(s, col)| {
        let navigate = use_navigate();
        let click = move |_| {
            let Some(s) = s.clone() else {
                return;
            };
            let mut query_map = query_map.get_untracked();
            if s == query_map.get_key_with_default(sort) {
                let toggle_order = match query_map.get_key_with_default(order) {
                    grpc::Order::Ascend => grpc::Order::Descend,
                    grpc::Order::Descend => grpc::Order::Ascend,
                };
                query_map.set_key(order, Some(toggle_order));
            } else {
                query_map.set_key(order, None);
                query_map.set_key(sort, Some(s));
            }
            navigate(
                &query_map.to_url(),
                NavigateOptions {
                    scroll: true,
                    ..Default::default()
                },
            );
        };
        view! {
            <th>
                <button on:click=click>{col}</button>
            </th>
        }
    });
    let rows = rows
        .into_iter()
        .map(|(href, cols)| view! { <Row cols href /> })
        .collect_view();
    view! {
        <table class=tw_join!("w-full grid gap-x-4", class)>
            <thead class="grid col-span-full grid-cols-subgrid font-bold text-base border-b-2 border-black-400 bg-black-900 p-4">
                <tr class="contents">{headers}</tr>
            </thead>
            <tbody class="contents">{rows}</tbody>
        </table>
    }
}

#[component]
fn Row<const N: usize>(
    cols: [View; N],
    href: impl ToHref + 'static,
) -> impl IntoView {
    let cols = cols
        .map(|v| view! { <td class="my-auto">{v}</td> })
        .collect_view();
    view! {
        <A class="grid col-span-full grid-cols-subgrid even:bg-black-900 text-sm p-4" href>
            <tr class="grid col-span-full grid-cols-subgrid">{cols}</tr>
        </A>
    }
}
