use leptos::*;
use leptos_router::*;
use tailwind_fuse::*;

use crate::utils::*;

#[component]
pub fn PaginateTableWithoutSort<const N: usize, H>(
    #[prop(into)] headers: [(Option<()>, View); N],
    #[prop(into)] rows: Result<Vec<(H, [View; N])>>,
    #[prop(into, optional)] class: String,
    #[prop(into)] order: ParamsMapKey<GrpcEnum<grpc::Order>>,
) -> impl IntoView
where
    H: ToHref + Clone + 'static,
{
    view! { <PaginateTable<N, DummyParamsMapValue, H> headers rows class order /> }
}

#[component]
pub fn PaginateTable<const N: usize, S, H>(
    #[prop(into)] headers: [(Option<S::Output>, View); N],
    #[prop(into)] rows: Result<Vec<(H, [View; N])>>,
    #[prop(into, optional)] class: String,
    #[prop(into, optional)] sort: Option<ParamsMapKey<S>>,
    #[prop(into)] order: ParamsMapKey<GrpcEnum<grpc::Order>>,
) -> impl IntoView
where
    S: ParamsMapValue + 'static,
    H: ToHref + Clone + 'static,
{
    let query_map = use_query_map();
    let headers = headers.map(|(s, col)| {
        let navigate = use_navigate();
        let Some(s) = s.clone() else {
            return view! { <th>{col}</th> };
        };

        let click = move |_| {
            let mut query_map = query_map.get_untracked();
            match sort {
                Some(sort) if s == query_map.get_key_with_default(sort) => {
                    let toggle_order =
                        match query_map.get_key_with_default(order) {
                            grpc::Order::Ascend => grpc::Order::Descend,
                            grpc::Order::Descend => grpc::Order::Ascend,
                        };
                    query_map.set_key(order, Some(toggle_order));
                }
                Some(sort) => {
                    query_map.set_key(order, None);
                    query_map.set_key(sort, Some(s.clone()));
                }
                None => {
                    let toggle_order =
                        match query_map.get_key_with_default(order) {
                            grpc::Order::Ascend => grpc::Order::Descend,
                            grpc::Order::Descend => grpc::Order::Ascend,
                        };
                    query_map.set_key(order, Some(toggle_order));
                }
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
    let rows = match rows {
        Err(v) if v.kind == ErrorKind::OutOfRange => Ok(view! {
            <tr class="grid col-span-full even:bg-black-900 text-sm text-center p-4">
                <p>No result</p>
            </tr>
        }.into_view()),
        
        Err(v)=>Err(v),
        Ok(v) => Ok(v
            .into_iter()
            .map(|(href, cols)| view! { <Row cols href /> })
            .collect_view()),
    };
    view! {
        <table class=tw_join!("grid gap-x-4", class)>
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
