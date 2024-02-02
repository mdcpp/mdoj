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
            let result: Result<ListProblemResponse> = async {
                Ok(ProblemSetClient::new(new_client().await?)
                    .list(ListProblemRequest {
                        size: 50,
                        offset: None,
                        request: Some(list_problem_request::Request::Create(
                            list_problem_request::Create {
                                sort_by: ProblemSortBy::UpdateDate.into(),
                                start_from_end: Some(false),
                            },
                        )),
                    })
                    .await?
                    .into_inner())
            }
            .await;
            match result {
                Ok(v) => Some(v),
                Err(e) => None,
            }
        },
    );

    view! {
        <div class="h-full flex flex-col items-center justify-between">
            <Transition fallback=move || {
                view! { <p>Loading</p> }
            }>
                <table class="text-text">
                    <thead class="text-left">
                        <tr>
                            <th>Title</th>
                            <th>AC Rate</th>
                            <th>Attempt</th>
                        </tr>
                    </thead>
                    <tbody class="text-lg">
                        {move || {
                            problems
                                .get()
                                .map(|v| {
                                    v.map(|v| {
                                        view! {
                                            {v
                                                .list
                                                .into_iter()
                                                .map(|info| {
                                                    view! {
                                                        <tr>
                                                            <td>
                                                                <A href=format!("/problem/{}", info.id.id)>{info.title}</A>
                                                            </td>
                                                            <td class="text-center">{info.ac_rate} %</td>
                                                            <td class="text-center">{info.submit_count}</td>
                                                        </tr>
                                                    }
                                                })
                                                .collect_view()}
                                        }
                                            .into_view()
                                    })
                                })
                        }}

                    </tbody>
                </table>
                <ul>
                    <li>-1</li>
                    <li>0</li>
                    <li>+1</li>
                </ul>
            </Transition>
        </div>
    }
}
