mod problem;

use grpc::backend::Paginator;
use leptos::{
    create_resource, html::P, Params, ReadSignal, Resource, SignalGet,
};
use leptos_router::{use_query, Params, ParamsError};
use serde::{Deserialize, Serialize};

/// how many page can user jump backward once
const BACKWARD_PAGE_LIMIT: usize = 6;
/// how many page can use jump forward once
const FORWARD_PAGE_LIMIT: usize = 6;

pub trait ParamWrapper
where
    Self: Sized,
{
    type Raw: Params + PartialEq + Clone;
    fn into_raw(self) -> Self::Raw;
    fn from_raw(raw: Self::Raw) -> Option<Self>;
    fn into_query(self) -> String
    where
        Self::Raw: Serialize,
    {
        let raw = self.into_raw();
        serde_qs::to_string(&raw).unwrap()
    }
}

/// query parameter for `use_navigation`
#[derive(Deserialize, Serialize, Clone, PartialEq, Params)]
struct RawPaginatorSession {
    /// split
    l: usize,
    /// page counter, indicating the page that would be fetched
    p: usize,
    /// page size
    s: usize,
    /// paginator session
    e: String,
}

/// abstracting single paginator(what backend provided) into paged paginator
///
/// It's named Paginator**Session** because it contains page_counter, page_size...
struct PaginatorSession {
    page_counter: usize,
    page_size: usize,
    split: usize,
    session: String,
}

impl ParamWrapper for PaginatorSession {
    type Raw = RawPaginatorSession;
    fn into_raw(self) -> Self::Raw {
        RawPaginatorSession {
            l: self.split,
            p: self.page_counter,
            s: self.page_size,
            e: self.session,
        }
    }
    fn from_raw(raw: Self::Raw) -> Option<Self> {
        Some(PaginatorSession {
            page_size: raw.s,
            page_counter: raw.p,
            split: raw.l,
            session: raw.e,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct RenderInfo<R: Fetcher> {
    pub forward_urls: Vec<R::URL>,
    pub backward_urls: Vec<R::URL>,
    pub list: Vec<R::Entry>,
}

pub trait Fetcher
where
    Self: 'static + Sized,
{
    type URL: Serialize + for<'a> Deserialize<'a>;
    /// element to paginate(UserInfo...)
    type Entry: Serialize + for<'a> Deserialize<'a>;
    /// parameter for first search(search by text...)
    type FirstSearch: ParamWrapper + Default;
    type Error: std::error::Error + Serialize + for<'a> Deserialize<'a>;
    fn get_size(request: &Self::FirstSearch) -> usize {
        10
    }
    /// search function for the first request
    ///
    /// return (list, paginator, remaining entry)
    async fn search(
        request: Self::FirstSearch,
    ) -> Result<(Vec<Self::Entry>, Paginator, usize), Self::Error>;
    /// search function for the subsequent request
    ///
    /// return (list, paginator, remaining entry)
    async fn fetch(
        session: Paginator,
        size: (usize, bool),
        offset: usize,
    ) -> Result<(Vec<Self::Entry>, Paginator, usize), Self::Error>;
    /// given url parameter, return url
    fn get_url_from_query(query: String) -> Self::URL;
    /// get render info from url
    async fn render_from_url() -> Resource<
        (
            Result<<Self::FirstSearch as ParamWrapper>::Raw, ParamsError>,
            Result<RawPaginatorSession, ParamsError>,
        ),
        Result<RenderInfo<Self>, Self::Error>,
    > {
        let search = use_query::<<Self::FirstSearch as ParamWrapper>::Raw>();
        let paginator = use_query::<RawPaginatorSession>();
        create_resource(
            move || (search.get(), paginator.get()),
            |(search, paginator)| async move {
                if let Some(search) =
                    search.ok().map(Self::FirstSearch::from_raw).flatten()
                {
                    Self::assume_search(search).await
                } else if let Some(session) =
                    paginator.ok().map(PaginatorSession::from_raw).flatten()
                {
                    Self::assume_paginate(session).await
                } else {
                    Self::assume_search(Self::FirstSearch::default()).await
                }
            },
        )
    }
    /// assume url contain paginator session
    async fn assume_paginate(
        session: PaginatorSession,
    ) -> Result<RenderInfo<Self>, Self::Error> {
        // FIXME: special condition when start==0, it is possible to have reversed request return insufficient entries
        let start = session.page_size * session.page_counter;
        let mut end = session.page_size * (session.page_counter + 1);
        if start < session.split && session.split < end {
            end = session.split;
        }
        let reverse = session.split > start;
        let offset = match reverse {
            true => session.split - end,
            false => start - session.split,
        };
        let (list, new_paginator, remain) = Self::fetch(
            Paginator::new(session.session),
            (end - start, reverse),
            offset,
        )
        .await?;

        let page_size = session.page_size;

        let forward_page_count = match reverse {
            true => (remain - (end - start)).div_ceil(page_size),
            false => remain.div_ceil(page_size),
        };
        let backward_page_count = match reverse {
            true => session.page_counter - 1,
            false => session.page_counter + 1,
        };

        let split = match reverse {
            true => session.split - list.len(),
            false => session.split + list.len(),
        };
        let session = new_paginator.session;

        Ok(RenderInfo {
            forward_urls: (1..=forward_page_count)
                .map(|p| {
                    PaginatorSession {
                        page_counter: backward_page_count + p,
                        page_size,
                        split,
                        session: session.clone(),
                    }
                    .into_query()
                })
                .map(Self::get_url_from_query)
                .take(FORWARD_PAGE_LIMIT)
                .collect(),
            backward_urls: (0..backward_page_count)
                .map(|page_counter| {
                    PaginatorSession {
                        page_counter,
                        page_size,
                        split,
                        session: session.clone(),
                    }
                    .into_query()
                })
                .map(Self::get_url_from_query)
                .take(BACKWARD_PAGE_LIMIT)
                .collect(),
            list,
        })
    }
    /// assume url contain search parameter(for first search)
    async fn assume_search(
        search: Self::FirstSearch,
    ) -> Result<RenderInfo<Self>, Self::Error> {
        let page_size = Self::get_size(&search);
        let (list, new_paginator, remain) = Self::search(search).await?;
        let split = list.len();
        Ok(RenderInfo {
            forward_urls: (1..=remain.div_ceil(page_size))
                .map(|page_counter| {
                    PaginatorSession {
                        page_counter,
                        page_size,
                        split,
                        session: new_paginator.clone().session,
                    }
                    .into_query()
                })
                .map(Self::get_url_from_query)
                .take(FORWARD_PAGE_LIMIT)
                .collect(),
            backward_urls: vec![],
            list,
        })
    }
}
