use leptos::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

use crate::{
    components::*,
    config::{use_token, WithToken},
    error::*,
    grpc::{problem_set_client::*, *},
    pages::{error_fallback, problems::toggle::Toggle},
};
const PAGESIZE: u64 = 10;

#[derive(Deserialize, Serialize, Default, Clone, PartialEq)]
pub enum SearchDeps {
    Text(String),
    Column(ProblemSortBy),
    #[default]
    None,
}

#[derive(Deserialize, Serialize, Default, Clone, PartialEq, Params)]
pub struct RawPager {
    /// session
    s: Option<String>,
    /// offset
    o: Option<u64>,
    /// direction
    d: Option<u8>,
    text: Option<String>,
    /// column search
    sort_by: Option<i32>,
    /// start from end
    e: Option<u8>,
    /// page_number
    p: Option<u64>,
}

/// Abtraction of paged(rather than cursor api of backend) content
///
/// It provides a clean interface to load next/previous page and store state
#[derive(Deserialize, Serialize, Default, Clone, PartialEq)]
pub struct Pager {
    // search deps
    deps: SearchDeps,
    session: Option<String>,
    direction: bool,
    offset: u64,
    // how many page before this page
    page_number: u64,
    start_from_end: bool,
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
            deps,
            session: value.s,
            direction: value.d.unwrap_or_default() == 1,
            offset: value.o.unwrap_or_default(),
            page_number: value.p.unwrap_or_default(),
            start_from_end: value.e.unwrap_or_default() == 1,
        }
    }
}

impl From<Pager> for RawPager {
    fn from(value: Pager) -> Self {
        macro_rules! on_option {
            ($e:expr,$t:ident) => {
                match ($e as $t) == $t::default() {
                    true => None,
                    false => Some($e as $t),
                }
            };
        }

        let mut text = None;
        let mut sort_by = None;
        match value.deps {
            SearchDeps::Text(text_) => text = Some(text_),
            SearchDeps::Column(sort_by_) => sort_by = Some(sort_by_ as i32),
            _ => {}
        };

        Self {
            s: value.session,
            o: on_option!(value.offset, u64),
            d: on_option!(value.direction, u8),
            text,
            sort_by,
            e: on_option!(value.start_from_end, u8),
            p: on_option!(value.page_number, u64),
        }
    }
}

impl Pager {
    fn into_query(self) -> String {
        let raw: RawPager = self.into();
        ["?", &*serde_qs::to_string(&raw).unwrap()].concat()
    }
    fn text_search(text: String) -> Self {
        Self {
            deps: SearchDeps::Text(text),
            ..Default::default()
        }
    }
    fn column_search(col: ProblemSortBy) -> Self {
        Self {
            deps: SearchDeps::Column(col),
            ..Default::default()
        }
    }
    fn from_end(mut self, start_from_end: bool) -> Self {
        self.start_from_end = start_from_end;
        self
    }
    fn get() -> Memo<Self> {
        Memo::new(move |_| {
            use_query::<RawPager>()
                .with(|v| v.clone().map(Into::into).ok())
                .unwrap_or_default()
        })
    }
    async fn get_respond(&mut self) -> Result<ListProblemResponse> {
        let size = match self.direction {
            true => -(PAGESIZE as i64),
            false => PAGESIZE as i64,
        };
        let (get_token, _) = use_token();
        let offset = Some(self.offset);
        let session = self.session.clone().map(|session| Paginator { session });
        let mut res = match &self.deps {
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
        .into_inner();

        if self.direction {
            res.list.reverse();
        }
        Ok(res)
    }
    fn get_queries(
        &self,
        new_session: String,
        remain: u64,
    ) -> (Vec<String>, Vec<String>) {
        let deps = self.deps.clone();
        let start_from_end = self.start_from_end;

        let mut previous_session = self.session.clone();
        let mut next_session = Some(new_session);
        if self.direction {
            std::mem::swap(&mut previous_session, &mut next_session);
        }

        let previous: Vec<_> = (0..self.page_number)
            .map(|p| {
                Self {
                    deps: deps.clone(),
                    session: previous_session.clone(),
                    direction: true,
                    offset: p * PAGESIZE,
                    page_number: self.page_number.saturating_sub(p + 1),
                    start_from_end,
                }
                .into_query()
            })
            .collect();
        let next: Vec<_> = (0..remain.div_ceil(PAGESIZE))
            .map(|p| {
                Self {
                    deps: deps.clone(),
                    session: next_session.clone(),
                    direction: false,
                    offset: p * PAGESIZE,
                    page_number: self.page_number.saturating_add(p + 1),
                    start_from_end,
                }
                .into_query()
            })
            .collect();
        (previous, next)
    }
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Default)]
struct RenderInfo {
    previous_queries: Vec<String>,
    list: Vec<ProblemInfo>,
    next_queries: Vec<String>,
}

impl RenderInfo {
    fn get() -> impl Fn() -> Resource<Pager, Result<Self>> {
        let pager = Pager::get();
        move || {
            create_resource(
                move || pager.get(),
                move |_| {
                    let mut pager = pager.get().clone();
                    async move {
                        pager.get_respond().await.map(|res| {
                            let list = res.list;
                            let (previous_queries, next_queries) =
                                pager.get_queries(res.next_session, res.remain);

                            Self {
                                list,
                                previous_queries,
                                next_queries,
                            }
                        })
                    }
                },
            )
        }
    }
}

#[component]
pub fn ProblemSearch() -> impl IntoView {
    let filter_text = create_rw_signal("".to_owned());
    let start_from_end = create_rw_signal(false);
    let sort_by = create_rw_signal((ProblemSortBy::AcRate as i32).to_string());

    // FIXME: What is this?
    let _submit = Memo::new(move |_| {
        let start_from_end = start_from_end.get();
        let text = filter_text.get();
        let sort_by: ProblemSortBy = sort_by
            .get()
            .parse::<i32>()
            .unwrap_or_default()
            .try_into()
            .unwrap_or(ProblemSortBy::UpdateDate);

        let pager = match text.is_empty() {
            true => Pager::column_search(sort_by),
            false => Pager::text_search(text),
        }
        .from_end(start_from_end);
        let query = pager.into_query();
        ["/problems", &query].concat()
    });

    view! {
        <div>
            <TextInput value=filter_text/>
            <span>
                <Toggle value=start_from_end/>
                Reverse
            </span>
            <Select value=sort_by>
                <SelectOption value="0">UpdateDate</SelectOption>
                <SelectOption value="1">CreateDate</SelectOption>
                <SelectOption value="2">AcRate</SelectOption>
                <SelectOption value="3">SubmitCount</SelectOption>
                <SelectOption value="4">Difficulty</SelectOption>
                <SelectOption value="6">Public</SelectOption>
            </Select>
        // a form with a text input and a dropdown list and a toggle

        </div>
    }
}
#[component]
pub fn ProblemList() -> impl IntoView {
    fn difficulty_color(difficulty: u32) -> impl IntoView {
        let color: &'static str = match difficulty {
            0..=1000 => "green",
            1001..=1500 => "orange",
            _ => "red",
        };
        view! {
            <span class=format!(
                "bg-{} text-{} text-xs font-medium me-2 px-2.5 py-0.5 rounded border border-{}",
                color,
                color,
                color,
            )>{difficulty}</span>
        }
    }

    let r = RenderInfo::get()();

    view! {
        <div>
            <Transition fallback=move || {
                view! { <p>Loading</p> }
            }>
                {move || {
                    r.get()
                        .map(|v| {
                            v
                                .map(|v| {
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
                                            <div class="table-row-group" style="line-height: 3rem">

                                                {v
                                                    .list
                                                    .into_iter()
                                                    .map(|info| {
                                                        view! {
                                                            <div class="table-row odd:bg-gray">
                                                                <div class="w-2/3 truncate table-cell">
                                                                    <A href=format!("/problem/{}", info.id.id)>{info.title}</A>
                                                                </div>
                                                                <div class="text-center hidden md:table-cell">
                                                                    {info.ac_rate} %
                                                                </div>
                                                                <div class="text-center hidden md:table-cell">
                                                                    {info.submit_count}
                                                                </div>
                                                                <div class="table-cell">
                                                                    {difficulty_color(info.difficulty)}
                                                                </div>
                                                            </div>
                                                        }
                                                    })
                                                    .collect_view()}

                                            </div>
                                        </div>
                                        <ul>

                                            {v
                                                .previous_queries
                                                .into_iter()
                                                .enumerate()
                                                .map(|(n, query)| {
                                                    view! {
                                                        <li>
                                                            <A href=format!("/problems{}", query)>back {n + 1} page</A>
                                                        </li>
                                                    }
                                                })
                                                .collect_view()}

                                        </ul>
                                        <ul>

                                            {v
                                                .next_queries
                                                .into_iter()
                                                .enumerate()
                                                .map(|(n, query)| {
                                                    view! {
                                                        <li>
                                                            <A href=format!("/problems{}", query)>next {n + 1} page</A>
                                                        </li>
                                                    }
                                                })
                                                .collect_view()}

                                        </ul>
                                    }
                                })
                        })
                }}

            </Transition>
        </div>
    }
}

#[component]
pub fn Problems() -> impl IntoView {
    view! {
        <div class="h-full container container-lg items-center justify-between text-lg">
            <ErrorBoundary fallback=error_fallback>
                <ProblemSearch/>
                <ProblemList/>
            </ErrorBoundary>
        </div>
    }
}
