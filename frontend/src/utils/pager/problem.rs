use grpc::backend::{Paginator, ProblemInfo};
use leptos::Params;
use leptos_router::{Params, ParamsError, ParamsMap};
use serde::{Deserialize, Serialize};

use crate::{
    error::{ErrorKind, Result},
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
    Column(i32, bool),
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
                s: Some(s),
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
            Some(sort_by) => SearchSession::Column(sort_by, reverse),
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
        todo!()
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
