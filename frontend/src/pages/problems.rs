use leptos::*;
use leptos_router::*;

use crate::{
    components::*,
    utils::{grpc::list_problem_request as lp_req, *},
};

async fn fetcher(
    paginator: Paginator<lp_req::Create>,
    size: u64,
    token: Option<String>,
) -> Result<(String, u64, Vec<grpc::ProblemInfo>)> {
    let request = match paginator {
        Paginator::Create(offset, create) => grpc::ListProblemRequest {
            size,
            offset,
            request: Some(lp_req::Request::Create(create)),
        },
        Paginator::Paginate(offset, paginator) => grpc::ListProblemRequest {
            size,
            offset,
            request: Some(lp_req::Request::Paginator(paginator)),
        },
    };
    let mut client =
        grpc::problem_client::ProblemClient::new(grpc::new_client());
    let list = client
        .list(request.with_optional_token(token))
        .await?
        .into_inner();
    Result::<_>::Ok((list.paginator, list.remain, list.list))
}

#[component]
pub fn Problems() -> impl IntoView {
    let mut problem_query = create_paginate_query(fetcher, Default::default());

    let params_map = use_query_map();
    let page = create_params_map_key("p", 0u32);
    let order = create_params_map_key("o", GrpcEnum(grpc::Order::Ascend));
    let sort = create_params_map_key("s", GrpcEnum(lp_req::Sort::Order));
    let text = create_params_map_key("t", "".to_owned());

    let info = Signal::derive(move || {
        params_map.with(|map| lp_req::Create {
            order: map.get_key_with_default(order).into(),
            query: Some(lp_req::Query {
                contest_id: None,
                sort_by: map.get_key(sort).map(|v| v.into()),
                text: map.get_key(text),
            }),
        })
    });

    let page_value = params_map.use_key_with_default(page);

    let query_result = problem_query.query(move || (page_value(), info()));
    let table = move || {
        query_result
            .data
            .get()
            .map(|d| d.map(|infos| view! { <Table infos sort order></Table> }))
    };

    let max_page = query_result.max_page;

    let navigate = use_navigate();
    let submit = move |e: ev::SubmitEvent, search: String| {
        e.prevent_default();
        let mut map = params_map.get_untracked();
        if search.is_empty() {
            map.set_key(text, None);
        } else {
            map.set_key(text, Some(search));
            map.set_key(page, None);
        }

        navigate(
            &map.to_url(),
            NavigateOptions {
                scroll: true,
                ..Default::default()
            },
        )
    };
    view! {
        <div class="container min-h-full flex flex-col items-center gap-4 py-10">
            <nav class="self-end">
                <SearchBar submit />
            </nav>
            <Suspense fallback=|| view! { "loading" }>
                <ErrorFallback>{table}</ErrorFallback>
            </Suspense>
            <PaginateNavbar size=4 page max_page />
        </div>
    }
}

#[component]
fn Table(
    infos: Vec<grpc::ProblemInfo>,
    sort: ParamsMapKey<GrpcEnum<lp_req::Sort>>,
    order: ParamsMapKey<GrpcEnum<grpc::Order>>,
) -> impl IntoView {
    let headers = [
        (Some(lp_req::Sort::Order), "Id".into_view()),
        (None, "Title".into_view()),
        (Some(lp_req::Sort::Difficulty), "Difficulty".into_view()),
        (Some(lp_req::Sort::SubmitCount), "Submit".into_view()),
        (Some(lp_req::Sort::AcRate), "Ac Rate".into_view()),
    ];
    let rows: Vec<_> = infos
        .into_iter()
        .map(|info| {
            (
                format!("/problem/{}", info.id),
                [
                    format!("{:04}", info.id).into_view(),
                    info.title.into_view(),
                    info.difficulty.into_view(),
                    info.submit_count.into_view(),
                    format!("{:.2}", info.ac_rate * 100.0).into_view(),
                ],
            )
        })
        .collect();

    view! {
        <PaginateTable
            class="grid-cols-[max-content_1fr_max-content_max-content_max-content]"
            headers
            rows
            sort
            order
        />
    }
}
