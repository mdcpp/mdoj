use crate::{
    components::*,
    grpc::{problem_set_client::*, *},
};
use anyhow::Result;
use leptos::*;
use leptos_router::*;

#[derive(Default, Clone, PartialEq, Params)]
struct Page {
    pager: Option<String>,
    offset: Option<usize>,
}

#[component]
pub fn Problems() -> impl IntoView {
    let page = use_params::<Page>();
    let problems = create_resource(
        move || page.with(|v| v.clone().unwrap_or_default()),
        |_| async move {
            Result::<_, String>::Ok(
                ProblemSetClient::new(new_client().await.map_err(|e| e.to_string())?)
                    .list(ListProblemRequest {
                        size: 50,
                        offset: None,
                        request: Some(list_problem_request::Request::Create(
                            list_problem_request::Create {
                                sort_by: ProblemSortBy::UpdateDate.into(),
                                reverse: false,
                            },
                        )),
                    })
                    .await
                    .map_err(|e| e.to_string())?
                    .into_inner(),
            )
        },
    );

    view! {
        <Transition fallback=move || {
            view! { <p>Loading</p> }
        }>
            {move || {
                problems
                    .get()
                    .map(|v| match v {
                        Ok(v) => {
                            view! {
                                <ul>
                                    {v
                                        .list
                                        .into_iter()
                                        .map(|info| view! { <li>{info.title}</li> })
                                        .collect_view()}
                                </ul>
                            }
                                .into_view()
                        }
                        Err(e) => view! { <p class="text-text">{e}</p> }.into_view(),
                    })
            }}

        </Transition>
    }
}
