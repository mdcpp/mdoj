use std::{borrow::BorrowMut, ops::DerefMut};

use leptos::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

use crate::{
    components::*,
    config::{self, use_token, WithToken},
    error::*,
    grpc::{problem_set_client::*, *},
    pages::problems::toggle::Toggle,
};

const PAGESIZE: i64 = 12;

#[derive(Deserialize, Serialize, Default, Clone, PartialEq, Params)]
pub struct Pager {
    text: Option<String>,
    sort_by: Option<i32>,
    offset: usize,
    page: Option<String>,
    start_from_end: Option<bool>,
}

impl Pager {
    //. store pager to url
    fn store(&self) {
        let navigate = leptos_router::use_navigate();

        let param = serde_qs::to_string(self).unwrap();

        navigate(
            &["/problems?".to_string(), param].concat(),
            Default::default(),
        );
    }
    /// load pager from url, return default if not found
    fn load() -> Pager {
        use_query::<Pager>()
            .with(|v| v.clone().ok())
            .unwrap_or_default()
    }
    async fn next(
        &self,
        size: i64,
        offset: u64,
    ) -> Result<(Self, Vec<ProblemInfo>)> {
        let (get_token, _) = use_token();
        let res = match &self.text {
            Some(text) => {
                ProblemSetClient::new(new_client().await?)
                    .search_by_text(
                        TextSearchRequest {
                            size,
                            offset: Some(offset),
                            request: Some(match &self.page {
                                Some(page) => {
                                    text_search_request::Request::Pager(
                                        Paginator {
                                            session: page.to_string(),
                                        },
                                    )
                                }
                                None => text_search_request::Request::Text(
                                    text.clone(),
                                ),
                            }),
                        }
                        .with_token(get_token),
                    )
                    .await?
            }
            None => {
                ProblemSetClient::new(new_client().await?)
                    .list(
                        ListProblemRequest {
                            size,
                            offset: Some(offset),
                            request: Some(match &self.page {
                                Some(page) => {
                                    list_problem_request::Request::Pager(
                                        Paginator {
                                            session: page.to_string(),
                                        },
                                    )
                                }
                                None => list_problem_request::Request::Create(
                                    list_problem_request::Create {
                                        sort_by: self.sort_by.unwrap_or(
                                            ProblemSortBy::UpdateDate as i32,
                                        ),
                                        start_from_end: self.start_from_end,
                                    },
                                ),
                            }),
                        }
                        .with_token(get_token),
                    )
                    .await?
            }
        }
        .into_inner();
        let mut list = res.list;
        if size < 0 {
            list.reverse();
        }

        // FIXME: pagaintor session at start of list if reversed
        let pager = Self {
            text: self.text.clone(),
            offset: self.offset + list.len(),
            page: Some(res.next_session),
            ..self.clone()
        };
        Ok((pager, list))
    }
}

fn difficulty_color(difficulty: u32) -> impl IntoView {
    let color: &'static str = match difficulty {
        0..=1000 => "green",
        1001..=1500 => "orange",
        _ => "red",
    };
    view! {
        <span class=format!("bg-{} text-{} text-xs font-medium me-2 px-2.5 py-0.5 rounded border border-{}", color, color, color)>
            {difficulty}
        </span>
    }
}

#[component]
pub fn ProblemSearch(set_pager: WriteSignal<Pager>) -> impl IntoView {
    // 1. add sort_by
    // 2. add search bar
    // 3. check reverse logic
    // 4. check hydration
    let search_text = create_rw_signal("".to_owned());
    let reverse = create_rw_signal(false);

    let submit =
        create_action(move |(search_text, reverse): &(String, bool)| {
            let serach_text = search_text.clone();

            let (get_token, _) = use_token();

            async move {
                // let mut problem_set = problem_set_client::ProblemSetClient::new(
                //     new_client().await?,
                // );
                // match search_text.is_empty(){
                //     true=>{
                //         let resp = problem_set
                //             .list(
                //                 ListProblemRequest {
                //                     size: 50,
                //                     offset: None,
                //                     request: Some(
                //                         list_problem_request::Request::Create(
                //                             list_problem_request::Create {
                //                                 sort_by: ProblemSortBy::UpdateDate
                //                                     .into(),
                //                                 start_from_end: Some(*reverse),
                //                             },
                //                         ),
                //                     ),
                //                 }
                //                 .with_token(get_token()),
                //             )
                //             .await?;
                //         let resp = resp.into_inner();
                //         Some(resp)
                //     }
                // }
            }
        });

    let disabled = Signal::derive(move || submit.pending()());

    view! {
        <div>
            <label for="search_text" class="text-text pb-2">
                List of problems
            </label>
            <TextInput
                id="search_text"
                value=search_text
                class="w-full"
                placeholder="Title tag1,tag2"
            />
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <Button kind="submit" class="w-full" disabled>
                    Search
                </Button>
                <div><Toggle value=reverse />Start from end</div>
            </div>
        </div>
    }
}

#[component]
pub fn ProblemList(pager: ReadSignal<Pager>) -> impl IntoView {
    let problems = create_resource(pager, |pager| async move {
        let (pager, list) = pager.next(-PAGESIZE, 0).await.unwrap();
        list
    });

    view! {
        <Transition fallback=move || {
            view! { <p>Loading</p> }
        }>
            <div class="table w-full table-auto">
                <div class="table-header-group text-left">
                    <div class="table-row">
                        <div class="table-cell">Title</div>
                        <div class="hidden md:table-cell">AC Rate</div>
                        <div class="hidden md:table-cell">Attempt</div>
                        <div class="table-cell">Difficulty</div>
                    </div>
                </div>
                {move || {
                    problems
                        .get()
                        .map(|v| {
                            view! {
                                <div class="table-row-group" style="line-height: 3rem">
                                {
                                    v
                                    .into_iter()
                                    .map(|info| {
                                        view! {
                                            <div class="table-row odd:bg-gray">
                                                <div class="w-2/3 truncate table-cell">
                                                    <A href=format!("/problem/{}", info.id.id)>{info.title}</A>
                                                </div>
                                                <div class="text-center hidden md:table-cell">{info.ac_rate} %</div>
                                                <div class="text-center hidden md:table-cell">{info.submit_count}</div>
                                                <div class="table-cell">{difficulty_color(info.difficulty)}</div>
                                            </div>
                                        }
                                    })
                                    .collect_view()
                                }
                                </div>
                                <ul>
                                    // {
                                    //     (-(params.offset/PAGESIZE)..(v.remain/PAGESIZE))
                                    //     .map(|i|{
                                    //         view!{
                                    //             <li>
                                    //                 {i}
                                    //             </li>
                                    //         }
                                    //     }).into_view()
                                    // }
                                    <li>-1</li>
                                    <li>0</li>
                                    <li>+1</li>
                                </ul>
                            }.into_view()
                        })
                }}
            </div>
        </Transition>
    }
}
#[component]
pub fn Problems() -> impl IntoView {
    let (pager, set_pager) = create_signal(Pager::default());

    view! {
        <div class="h-full container container-lg items-center justify-between text-lg">
            <ProblemSearch set_pager=set_pager/>
            <ProblemList pager=pager/>
        </div>
    }
}
