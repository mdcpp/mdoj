use chrono::{DateTime, Utc};
use leptos::*;
use leptos_router::{use_params, use_query_map, Params};

use crate::{
    components::*,
    utils::{grpc::list_submit_request as ls_req, *},
};

async fn fetcher(
    paginator: Paginator<Result<ls_req::Create>>,
    size: u64,
    token: Option<String>,
) -> Result<(String, u64, Vec<grpc::SubmitInfo>)> {
    let request = match paginator {
        Paginator::Create(offset, create) => grpc::ListSubmitRequest {
            size,
            offset,
            request: Some(ls_req::Request::Create(create?)),
        },
        Paginator::Paginate(offset, paginator) => grpc::ListSubmitRequest {
            size,
            offset,
            request: Some(ls_req::Request::Paginator(paginator)),
        },
    };
    let mut client = grpc::submit_client::SubmitClient::new(grpc::new_client());
    let list = client
        .list(request.with_optional_token(token))
        .await?
        .into_inner();
    Result::<_>::Ok((list.paginator, list.remain, list.list))
}

#[derive(Params, Debug, PartialEq, Eq, Clone, Hash, Default)]
struct SubmissionParams {
    id: i32,
}

#[component]
pub fn ProblemSubmissions() -> impl IntoView {
    let mut submit_query = create_paginate_query(fetcher, Default::default());
    let params = use_params::<SubmissionParams>();

    let query_map = use_query_map();
    let page = create_params_map_key("p", 0u32);
    let order = create_params_map_key("o", GrpcEnum(grpc::Order::Ascend));

    let info = Signal::derive(move || {
        query_map.with(|map| {
            Ok(ls_req::Create {
                order: map.get_key_with_default(order).into(),
                problem_id: Some(params()?.id),
            })
        })
    });

    let page_value = query_map.use_key_with_default(page);

    let headers = [
        (Some(()), "When".into_view()),
        (None, "Id".into_view()),
        (None, "State".into_view()),
        (None, "Score".into_view()),
    ];

    let query_result = submit_query.query(move || (page_value(), info()));
    let table = move || {
        query_result.data.get().map(|v| {
            v.map(|v| {
                v.into_iter()
                    .map(|info| {
                        
                        let when:DateTime<Utc>=info.upload_time.into();
                        (
                            format!("/submission/{}", info.id),
                            [
                                when.format("%Y/%m/%d %H:%M:%S").to_string().into_view(),
                                format!("{:04}", info.id).into_view(),
                                view! { <StateBadge state=info.state.code.try_into().unwrap() /> }.into_view(),
                                info.score.into_view(),
                            ],
                        )
                    })
                    .collect::<Vec<_>>()
            })
        }).map(|rows|view! {
            <PaginateTableWithoutSort
                class="grid-cols-[1fr_max-content_max-content_max-content] text-center w-full"
                headers=headers.clone()
                rows
                order
            />
        })
    };

    let max_page = query_result.max_page;

    view! {
        <div class="container">
            <div class="overflow-y-auto relative size-full">
                <div class="flex flex-col items-center justify-between gap-4 py-10 px-4 absolute size-full top-0 left-0">
                    <Suspense fallback=|| view! { "loading" }>
                        <ErrorFallback clone:table>{table}</ErrorFallback>
                    </Suspense>
                    <PaginateNavbar size=4 page max_page />
                </div>
            </div>
        </div>
    }
}
