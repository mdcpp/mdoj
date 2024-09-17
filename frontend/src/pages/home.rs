use chrono::{DateTime, Utc};
use leptos::*;
use leptos_router::use_query_map;

use crate::{
    components::*,
    utils::{grpc::list_announcement_request as la_req, *},
};

async fn fetcher(
    paginator: Paginator<Result<la_req::Create>>,
    size: u64,
    token: Option<String>,
) -> Result<(String, u64, Vec<grpc::AnnouncementInfo>)> {
    let request = match paginator {
        Paginator::Create(offset, create) => grpc::ListAnnouncementRequest {
            size,
            offset,
            request: Some(la_req::Request::Create(create?)),
        },
        Paginator::Paginate(offset, paginator) => {
            grpc::ListAnnouncementRequest {
                size,
                offset,
                request: Some(la_req::Request::Paginator(paginator)),
            }
        }
    };
    let mut client =
        grpc::announcement_client::AnnouncementClient::new(grpc::new_client());
    let list = client
        .list(request.with_optional_token(token))
        .await?
        .into_inner();
    Result::<_>::Ok((list.paginator, list.remain, list.list))
}

#[component]
pub fn Home() -> impl IntoView {
    let mut submit_query = create_paginate_query(fetcher, Default::default());

    let query_map = use_query_map();
    let page = create_params_map_key("p", 0u32);
    let order = create_params_map_key("o", GrpcEnum(grpc::Order::Ascend));

    let info = Signal::derive(move || {
        query_map.with(|map| {
            Ok(la_req::Create {
                order: map.get_key_with_default(order).into(),
                query: Some(la_req::Query {
                    sort_by: None,
                    text: None,
                    contest_id: None,
                }),
            })
        })
    });

    let page_value = query_map.use_key_with_default(page);

    let headers = [
        (None, "Id".into_view()),
        (None, "Announcement Title".into_view()),
        (Some(()), "When".into_view()),
    ];

    let query_result = submit_query.query(move || (page_value(), info()));
    let table = move || {
        query_result
            .data
            .get()
            .map(|v| {
                v.map(|v| {
                    v.into_iter()
                        .map(|info| {
                            let when: DateTime<Utc> = info.update_date.into();
                            (
                                format!("/announcement/{}", info.id),
                                [
                                    format!("{:04}", info.id).into_view(),
                                    info.title.into_view(),
                                    when.format("%Y/%m/%d %H:%M:%S")
                                        .to_string()
                                        .into_view(),
                                ],
                            )
                        })
                        .collect::<Vec<_>>()
                })
            })
            .map(|rows| {
                view! {
                    <PaginateTableWithoutSort
                        class="grid-cols-[max-content_1fr_max-content_max-content] w-full"
                        headers=headers.clone()
                        rows
                        order
                    />
                }
            })
    };

    let max_page = query_result.max_page;

    view! {
        <div class="container grow flex flex-col items-center justify-between gap-4 py-10">
            <Transition fallback=|| view! { "loading" }>
                <ErrorFallback clone:table>{table}</ErrorFallback>
            </Transition>
            <PaginateNavbar size=4 page max_page />
        </div>
    }
}
