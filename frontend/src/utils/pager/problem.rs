use grpc::backend::{Paginator, ProblemInfo};
use leptos::{Params, SignalGetUntracked};
use leptos_router::{Params, ParamsError, ParamsMap};
use serde::{Deserialize, Serialize};

use crate::{
    error::{ErrorKind, Result, *},
    grpc::{problem_set_client::*, *},
    pages::error_fallback,
    session::use_token,
    utils::pager::{Fetcher, ParamWrapper},
};

#[derive(Deserialize, Serialize, Default, Clone, PartialEq, Params)]
pub struct RawSearchSession {
    /// text search
    t: Option<String>,
    /// column search
    s: Option<i32>,
    /// start from end
    r: Option<bool>,
}

pub enum SearchSession {
    Text(String),
    Column(ProblemSortBy, bool),
    List(bool),
}

impl Default for SearchSession {
    fn default() -> Self {
        Self::List(false)
    }
}

impl ParamWrapper for SearchSession {
    type Raw = RawSearchSession;
    fn into_raw(self) -> Self::Raw {
        match self {
            SearchSession::Text(x) => RawSearchSession {
                t: Some(x),
                ..Default::default()
            },
            SearchSession::Column(s, r) => RawSearchSession {
                s: Some(s as i32),
                r: Some(r),
                ..Default::default()
            },
            SearchSession::List(r) => RawSearchSession {
                r: Some(r),
                ..Default::default()
            },
        }
    }

    fn from_raw(raw: Self::Raw) -> Option<Self> {
        if let Some(x) = raw.t {
            return Some(SearchSession::Text(x));
        }
        let reverse = raw.r?;
        Some(match raw.s {
            Some(sort_by) => SearchSession::Column(
                sort_by.try_into().unwrap_or_default(),
                reverse,
            ),
            None => SearchSession::List(reverse),
        })
    }
}

pub struct ProblemFetcher;

impl Fetcher for ProblemFetcher {
    type URL = String;
    type Entry = ProblemInfo;
    type FirstSearch = SearchSession;
    type Error = ErrorKind;
    async fn search(
        request: Self::FirstSearch,
    ) -> Result<(Vec<Self::Entry>, Paginator, usize)> {
        let token = use_token().get_untracked();

        let res = match request {
            SearchSession::Text(xtext) => todo!(),
            // ProblemSetClient::new(new_client().await?)
            // .search_by_text(
            //     TextSearchRequest {
            //         size: Self::get_size(&request) as i64,
            //         offset: None,
            //         request: Some(
            //             text_search_request::Request::Create(
            //                 list_problem_request::Create {
            //                     sort_by: column as i32,
            //                     start_from_end: Some(start_from_end),
            //                 },
            //             ),
            //         ),
            //     }
            //         .with_optional_token(token),
            // )
            // .await,
            SearchSession::Column(column, start_from_end) => {
                ProblemSetClient::new(new_client().await?)
                    .list(
                        ListProblemRequest {
                            size: Self::get_size(&request) as i64,
                            offset: None,
                            request: Some(
                                list_problem_request::Request::Create(
                                    list_problem_request::Create {
                                        sort_by: column as i32,
                                        start_from_end: Some(start_from_end),
                                    },
                                ),
                            ),
                        }
                        .with_optional_token(token),
                    )
                    .await
            }

            SearchSession::List(_) => todo!(),
        }?
        .into_inner();
        Ok((
            res.list,
            Paginator::new(res.next_session),
            res.remain as usize,
        ))
    }
    async fn fetch(
        session: Paginator,
        size: (usize, bool),
        offset: usize,
    ) -> Result<(Vec<Self::Entry>, Paginator, usize)> {
        todo!()
    }
    fn get_url_from_query(query: String) -> Self::URL {
        ["/problems?", &*query].concat()
    }
}
