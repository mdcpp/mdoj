use std::{borrow::BorrowMut, default, ops::DerefMut};
use std::rc::Rc;

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
use crate::pages::error_fallback;
const PAGESIZEU64: u64 = 10;
const PAGESIZEUSIZE: usize = 10;
const PAGESIZEI64: i64 = 10;

#[derive(Deserialize, Serialize, Default, Clone, PartialEq)]
pub enum SearchDeps {
    Text(String),
    Column(ProblemSortBy),
    #[default]
    None,
}

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
    se: Option<u32>,
}

/// Abtraction of paged(rather than cursor api of backend) content
///
/// It provides a clean interface to load next/previous page and store state
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
            start_from_end: value.se.unwrap_or_default()==1,
        }
    }
}


impl From<Pager> for RawPager {
    fn from(value: Pager) -> Self {
        let mut text = None;
        let mut sort_by = None;
        match value.deps {
            SearchDeps::Text(text_) => text = Some(text_),
            SearchDeps::Column(sort_by_) => sort_by = Some(sort_by_ as i32),
            _ => {}
        };

        let mut tp = None;
        let mut lp = None;
        if let Some(session) = value.session {
            match value.at_end {
                true => lp = Some(session),
                false => tp = Some(session),
            }
        }

        Self {
            tp,
            lp,
            to: Some(value.offset.0),
            lo: Some(value.offset.1),
            text,
            sort_by,
            se: Some(value.start_from_end as u32),
        }
    }
}

impl Pager {
    pub fn search(deps: SearchDeps,start_from_end:bool)->Self{
        let self_=Self{
            deps,
            session: None,
            at_end: false,
            offset: (0, 0),
            start_from_end,
        };
        self_.store();
        self_
    }
    pub fn is_default(&self)->bool{
        self.offset.0==0
    }
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
    /// move to next page
    ///
    /// note that use can pass pages of 0 to fetch the current page from server again
    ///
    /// * `pages` - how many pages to move.
    pub async fn next(&mut self, pages: i64) -> Result<RenderInfo> {
        /// FIXME: add bound check
        let mut offset=(pages.abs()as u64)*PAGESIZEU64;
        if (pages>0)^self.at_end{
            offset+=self.offset.1-self.offset.0;
        }
        let mut res = self
            .raw_next(
                PAGESIZEI64,
                offset,
                self.session.clone().map(|session| Paginator { session }),
            )
            .await?;

        match pages{
            0 if self.at_end =>res.list.reverse(),
            x if x<0=>res.list.reverse(),
            _=>(),
        }
        let res_len = res.list.len() as u64;

        if pages.is_positive() {
            self.offset.0 = self.offset.1;
            self.offset.1 += res_len;
        } else {
            self.offset.1 = self.offset.0;
            self.offset.0 = self.offset.0.saturating_sub(res_len);
        }

        self.session=Some(res.next_session);
        self.store();

        Ok(RenderInfo {
            // FIXME: add bound check for underflow entity
            previous_page: (self.offset.0) as u64 / PAGESIZEU64,
            list: res.list,
            next_page: (res.remain + PAGESIZEU64 - 1) / PAGESIZEU64,
        })
    }
    // pub async fn load_and_next(pages: i64) -> Result<RenderInfo>{
    //     let mut self_=Self::load();
    //     let list=self_.next(pages).await?;
    //     Ok(list)
    // }
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Default)]
struct RenderInfo {
    previous_page: u64,
    list: Vec<ProblemInfo>,
    next_page: u64,
}

#[component]
pub fn ProblemList(render: ReadSignal<Result<RenderInfo>>, next_action:Action<i64,()>) -> impl IntoView {
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

    let list= move|| {
        view! {
            <div class="table-row-group" style="line-height: 3rem">
            {
                move||render.get().map(|v|
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
                )
            }
            </div>
            <ul>
            {
                move||render.get().clone().map(|v|
                    (1..=(v.previous_page))
                    .map(|i|{
                        view!{
                            <li on:click=move |_| next_action.dispatch(-(i as i64))>
                                back {i}th page
                            </li>
                        }
                    }).collect_view()
                )
            }
            </ul>
            <ul>
            {
                move||render.get().clone().map(|v|
                    (1..=(v.next_page))
                    .map(|i|{
                        view!{
                            <li on:click=move |_| next_action.dispatch(i as i64)>
                                advance {i}th page
                            </li>
                        }
                    }).collect_view()
                )
            }
            </ul>
        }.into_view()
    };
    view! {
        <div class="table w-full table-auto">
            <div class="table-header-group text-left">
                <div class="table-row">
                    <div class="table-cell">Title</div>
                    <div class="hidden md:table-cell">AC Rate</div>
                    <div class="hidden md:table-cell">Attempt</div>
                    <div class="table-cell">Difficulty</div>
                </div>
            </div>
            {list}
        </div>
    }
}

#[component]
pub fn ProblemSearch(set_render: WriteSignal<Result<RenderInfo>>) -> impl IntoView {
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
pub fn Problems() -> impl IntoView {
    let (render, set_render) = create_signal(Ok(RenderInfo::default()));

    let render_resource =create_resource(||(), move|_| {
        let set_render=set_render.clone();
        let mut pager = Pager::load();
        let pages = match pager.is_default() {
            true => 1,
            false => 0
        };
        async move {
            set_render.set(pager.next(pages).await);
        }
    });

    let next_action=create_action(move |(pages):&(i64)|{
        let mut pager=Pager::load();
        let pages=*pages;
        async move{
            set_render.set(pager.next(pages).await);
        }
    });

    view! {
        <div class="h-full container container-lg items-center justify-between text-lg">
            <ProblemSearch set_render=set_render/>
            <Transition fallback=move || {
                view! { <p>Loading</p> }
            }>
                <ErrorBoundary fallback=error_fallback>
                {
                    render_resource.get().map(|_|view!{<ProblemList render=render next_action=next_action/>})
                }
                </ErrorBoundary>
            </Transition>
        </div>
    }
}
