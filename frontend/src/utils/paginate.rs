use std::{
    fmt::Debug,
    future::Future,
    hash::{DefaultHasher, Hash, Hasher},
};

use leptos::*;
use leptos_query::*;

use super::{config::*, error::*, session::*};

mod private {
    use super::*;

    pub trait PaginateQueryKey: Debug + Clone + Hash + PartialEq {}

    impl<T> PaginateQueryKey for T where T: Debug + Clone + Hash + PartialEq {}

    #[derive(Debug, Clone)]
    pub struct InnerPaginateQueryKey<Info: PaginateQueryKey + 'static> {
        pub page: i64,
        pub info: Info,
        pub prev_paginator: Option<(i64, String)>,
    }

    impl<Info: PaginateQueryKey + 'static> Hash for InnerPaginateQueryKey<Info> {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            self.page.hash(state);
            self.info.hash(state);
        }
    }

    impl<T: PaginateQueryKey + 'static> PartialEq for InnerPaginateQueryKey<T> {
        fn eq(&self, other: &Self) -> bool {
            self.page == other.page && self.info == other.info
        }
    }

    impl<Info: PaginateQueryKey + 'static> Eq for InnerPaginateQueryKey<Info> {}
}
use private::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Paginator<T> {
    /// offset item count `.0` from beginning
    Create(i64, T),
    /// offset item count `.0` from `.1` paginator
    Paginate(i64, String),
}

/// `fetcher` is `fn(Paginator<Info>, page index, token) -> (next paginator, remain item count, Data)`
pub fn create_paginate_query<T, Fu, Info, Data>(
    fetcher: T,
    options: QueryOptions<Result<Data>>,
) -> PaginateQuery<Info, Data>
where
    T: Fn(Paginator<Info>, u64, Option<String>) -> Fu + 'static,
    Fu: Future<Output = Result<(String, u64, Data)>> + 'static,
    Info: PaginateQueryKey + 'static,
    Data: QueryValue + 'static,
    InnerPaginateQueryKey<Info>: QueryKey + 'static,
    Result<Data>: QueryValue + 'static,
{
    let max_page = create_rw_signal(0);

    let prev_paginator = store_value(None);

    let page_size = frontend_config().page_size;
    let token = use_token();

    let fetcher = move |query_token: InnerPaginateQueryKey<Info>| {
        let (paginator, next_paginator_index) =
            if let Some((index, paginator)) = query_token.prev_paginator {
                let delta = query_token.page - index;
                (
                    Paginator::Paginate(delta * page_size as i64, paginator),
                    query_token.page + (!delta.is_negative()) as i64,
                )
            } else {
                (
                    Paginator::Create(
                        query_token.page * page_size as i64,
                        query_token.info,
                    ),
                    query_token.page + 1,
                )
            };
        let fu = fetcher(paginator, page_size as u64, token.get_untracked());
        async move {
            let (next_paginator, remain_item, data) = fu.await?;

            let remain_page = remain_item.div_ceil(page_size as u64) as u32;

            // FIXME: move this to result, bc this only trigger when refetch
            if remain_page != 0 {
                prev_paginator
                    .set_value(Some((next_paginator_index, next_paginator)));
            }

            let page = query_token.page as u32 + remain_page;
            if max_page.get_untracked() < page {
                max_page.set(page);
            }

            Ok(data)
        }
    };
    let scope = create_query(fetcher, options);

    PaginateQuery {
        version: Default::default(),
        prev_paginator,
        max_page,
        scope,
    }
}

#[derive(Clone)]
pub struct PaginateQuery<Info, Data>
where
    Info: PaginateQueryKey + 'static,
    InnerPaginateQueryKey<Info>: QueryKey + 'static,
    // (page index, next paginator, remain page count, Data)
    Result<Data>: QueryValue + 'static,
{
    version: StoredValue<u64>,
    prev_paginator: StoredValue<Option<(i64, String)>>,
    max_page: RwSignal<u32>,
    scope: QueryScope<InnerPaginateQueryKey<Info>, Result<Data>>,
}

#[derive(Clone, Copy)]
pub struct PaginateQueryResult<Data: 'static> {
    /// Should be called inside of a [`Transition`](leptos::Transition) or [`Suspense`](leptos::Suspense) component.
    pub data: Signal<Option<Result<Data>>>,
    /// How max page count
    pub max_page: Signal<u32>,
}

impl<Info, Data> PaginateQuery<Info, Data>
where
    Info: PaginateQueryKey + 'static,
    InnerPaginateQueryKey<Info>: QueryKey + 'static,
    Result<Data>: QueryValue + 'static,
{
    /// `key` is `fn() -> (page index, Info)`
    pub fn query(
        &mut self,
        key: impl Fn() -> (u32, Info) + 'static,
    ) -> PaginateQueryResult<Data> {
        let version = self.version;
        let prev_paginator = self.prev_paginator;
        let max_page = self.max_page;
        let current_page = create_rw_signal(0);

        let query = self.scope.use_query(move || {
            let (page, info) = key();

            // cache invalidation
            let mut hasher = DefaultHasher::new();
            info.hash(&mut hasher);
            let hash = hasher.finish();
            if hash != version() {
                version.set_value(hash);
                prev_paginator.set_value(None);
                max_page.set(0);
            }

            current_page.set(page);
            InnerPaginateQueryKey {
                page: page as i64,
                info,
                prev_paginator: prev_paginator(),
            }
        });

        let data = Signal::derive(move || {
            (query.data)().map(|v| {
                let data = v?;

                Result::<_>::Ok(data)
            })
        });
        let max_page = self.max_page.into_signal();

        PaginateQueryResult { data, max_page }
    }
}
