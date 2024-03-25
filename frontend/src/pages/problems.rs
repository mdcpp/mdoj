use std::{borrow::BorrowMut, default, ops::DerefMut};

use leptos::{html::s, *};
use leptos_router::*;
use serde::{Deserialize, Serialize};

use crate::{
    components::*,
    config::{self, use_token, WithToken},
    error::*,
    grpc::{problem_set_client::*, *},
    pages::problems::toggle::Toggle,
};

const PAGESIZEu64: u64 = 12;
const PAGESIZEusize: usize = 12;
const PAGESIZEi64: i64 = 12;

#[derive(Deserialize, Serialize, Default, Clone, PartialEq, Params)]
pub struct RawPager {
    /// trailing pager session
    tp: Option<String>,
    /// leading pager session
    lp: Option<String>,
    /// trailing pager offset
    to: Option<u64>,
    /// leading pager offset
    lo: Option<u64>,
    /// text search
    text: Option<String>,
    /// column search
    sort_by: Option<i32>,
    /// start from end
    se: Option<bool>,
}

#[derive(Deserialize, Serialize, Default, Clone, PartialEq)]
pub enum SearchDeps {
    Text(String),
    Column(ProblemSortBy),
    #[default]
    None,
}
/// Abtraction of paged(rather than cursor) content
#[derive(Deserialize, Serialize, Default, Clone, PartialEq)]
pub struct Pager {
    // search deps
    deps: SearchDeps,
    session: Option<String>,
    /// whether to session is at end
    at_end: bool,
    offset: (u64, u64),
    start_from_end: bool,
}

impl From<Pager> for RawPager {
    fn from(value: Pager) -> Self {
        Self {
            tp: todo!(),
            lp: todo!(),
            to: todo!(),
            lo: todo!(),
            text: todo!(),
            sort_by: todo!(),
            se: todo!(),
        }
    }
}

/// merge two option
macro_rules! merge {
    ($a:expr,$b:expr) => {
        match $a {
            Some(x) => Some(x),
            None => $b,
        }
    };
}

impl From<RawPager> for Pager {
    fn from(value: RawPager) -> Self {
        let deps = match (value.text, value.sort_by) {
            (Some(text), _) => SearchDeps::Text(text),
            (_, Some(sort_by)) => {
                SearchDeps::Column(sort_by.try_into().unwrap_or_default())
            }
            _ => SearchDeps::None,
        };
        Self {
            at_end: value.tp.is_some(),
            deps,
            session: merge!(value.tp, value.lp),
            offset: (
                value.to.unwrap_or_default(),
                value.lo.unwrap_or_default(),
            ),
            start_from_end: value.se.unwrap_or_default(),
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Default)]
struct RenderInfo {
    previous_page: u64,
    list: Vec<ProblemInfo>,
    next_page: u64,
}

impl Pager {
    /// store pager to url
    fn store(&self) {
        let navigate = leptos_router::use_navigate();
        let raw: RawPager = self.clone().into();
        let param = serde_qs::to_string(&raw).unwrap();

        navigate(
            &["/problems?".to_string(), param].concat(),
            Default::default(),
        );
    }
    /// load pager from url, return default if not found
    fn load() -> Pager {
        use_query::<RawPager>()
            .with(|v| v.clone().map(Into::into).ok())
            .unwrap_or_default()
    }
    /// emit rpc with corresponding endpoint and search parameter(if session is empty)
    async fn raw_next(
        &mut self,
        size: i64,
        offset: u64,
        session: Option<Paginator>,
    ) -> Result<ListProblemResponse> {
        let (get_token, _) = use_token();
        let offset = Some(offset);
        Ok(match &self.deps {
            SearchDeps::Text(text) => {
                ProblemSetClient::new(new_client().await?)
                    .search_by_text(
                        TextSearchRequest {
                            size,
                            offset,
                            request: Some(match session {
                                Some(session) => {
                                    text_search_request::Request::Pager(session)
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
            SearchDeps::Column(col) => {
                ProblemSetClient::new(new_client().await?)
                    .list(
                        ListProblemRequest {
                            size,
                            offset,
                            request: Some(match session {
                                Some(session) => {
                                    list_problem_request::Request::Pager(
                                        session,
                                    )
                                }
                                None => list_problem_request::Request::Create(
                                    list_problem_request::Create {
                                        sort_by: *col as i32,
                                        start_from_end: Some(
                                            self.start_from_end,
                                        ),
                                    },
                                ),
                            }),
                        }
                        .with_token(get_token),
                    )
                    .await?
            }
            SearchDeps::None => {
                ProblemSetClient::new(new_client().await?)
                    .list(
                        ListProblemRequest {
                            size,
                            offset,
                            request: Some(match session {
                                Some(session) => {
                                    list_problem_request::Request::Pager(
                                        session,
                                    )
                                }
                                None => list_problem_request::Request::Create(
                                    list_problem_request::Create {
                                        sort_by: ProblemSortBy::UpdateDate
                                            as i32,
                                        start_from_end: Some(
                                            self.start_from_end,
                                        ),
                                    },
                                ),
                            }),
                        }
                        .with_token(get_token),
                    )
                    .await?
            }
        }
        .into_inner())
    }
    async fn next(&mut self, pages: i64) -> Result<RenderInfo> {
        let mut offset = 0;

        if pages.is_negative() ^ self.at_end {
            offset = self.offset.1 - self.offset.0;
        }
        let mut res = self
            .raw_next(
                pages * PAGESIZEi64,
                offset,
                self.session.clone().map(|session| Paginator { session }),
            )
            .await?;

        if pages.is_negative() {
            res.list.reverse();
        }
        let res_len = res.list.len() as u64;

        if pages.is_positive() {
            self.offset.0 = self.offset.1;
            self.offset.1 += res_len;
        } else {
            self.offset.1 = self.offset.0;
            self.offset.0 = self.offset.0.saturating_sub(res_len);
        }
        Ok(RenderInfo {
            // FIXME: add bound check for underflow entity
            previous_page: (self.offset.0) as u64 / PAGESIZEu64,
            list: res.list,
            next_page: (res.remain + PAGESIZEu64 - 1) / PAGESIZEu64,
        })
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
    let problems = create_resource(pager, |mut pager| async move {
        let render = pager.next(1).await.unwrap();
        pager.store();
        render
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
                                    v.list
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
                                {
                                    (0..(v.previous_page))
                                    .map(|i|{
                                        view!{
                                            <li>
                                                back {i}th page
                                            </li>
                                        }
                                    }).collect_view()
                                }
                                </ul>
                                <ul>
                                {
                                    (0..(v.next_page))
                                    .map(|i|{
                                        view!{
                                            <li>
                                                advance {i}th page
                                            </li>
                                        }
                                    }).collect_view()
                                }
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
