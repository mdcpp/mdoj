use leptos::*;
use leptos_router::*;

use crate::{
    components::*,
    config::{use_token, WithToken},
    error::*,
    grpc::{problem_set_client::*, *},
};

#[derive(Default, Clone, PartialEq, Params)]
struct Page {
    pager: Option<String>,
    offset: Option<usize>,
}

fn difficulty_color(difficulty:u32)->impl IntoView{
    match difficulty{
        0..=1000=>view! {
            <span class="text-white">Easy - {difficulty}</span>
        },
        1000..=1500=>view! {
            <span class="text-orange">Medium - {difficulty}</span>
        },
        _=>view! {
            <span class="text-red">Hard - {difficulty}</span>
        }
    }
}

#[component]
pub fn Problems() -> impl IntoView {
    let params = use_params::<Page>();
    let page = move || params.with(|v| v.clone().unwrap_or_default());
    let (token, _) = use_token();
    let page_and_token = move || (page(), token());

    let problems =
        create_resource(page_and_token, |(page, token)| async move {
            let result: Result<ListProblemResponse> = async {
                Ok(ProblemSetClient::new(new_client().await?)
                    .list(
                        ListProblemRequest {
                            size: 50,
                            offset: None,
                            request: Some(
                                list_problem_request::Request::Create(
                                    list_problem_request::Create {
                                        sort_by: ProblemSortBy::UpdateDate
                                            .into(),
                                        start_from_end: Some(false),
                                    },
                                ),
                            ),
                        }
                        .with_token(token),
                    )
                    .await?
                    .into_inner())
            }
            .await;
            match result {
                Ok(v) => Some(v),
                Err(e) => None,
            }
        });

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
                            <th>Difficulty</th>
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
                                                        <tr class="odd:bg-gray">
                                                            <td>
                                                                <A href=format!("/problem/{}", info.id.id)>{info.title}</A>
                                                            </td>
                                                            <td class="text-center">{info.ac_rate} %</td>
                                                            <td class="text-center">{info.submit_count}</td>
                                                            <td>{difficulty_color(info.difficulty)}</td>
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
