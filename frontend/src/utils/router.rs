use std::{marker::PhantomData, str::FromStr};

use leptos::*;
use leptos_router::*;

pub trait QueryType {
    type Output: Clone + PartialEq + 'static;
    fn inner(self) -> Self::Output;
    fn convert_to_type(s: &str) -> Option<Self::Output>;
    fn convert_to_string(o: Self::Output) -> String;
}

impl<T> QueryType for T
where
    T: FromStr + ToString + Clone + PartialEq + 'static,
{
    type Output = T;

    fn inner(self) -> Self::Output {
        self
    }

    fn convert_to_type(s: &str) -> Option<Self::Output> {
        s.parse().ok()
    }

    fn convert_to_string(o: Self::Output) -> String {
        o.to_string()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GrpcEnum<T>(pub T);

impl<T> QueryType for GrpcEnum<T>
where
    T: TryFrom<i32> + Into<i32> + Clone + PartialEq + 'static,
{
    type Output = T;

    fn inner(self) -> Self::Output {
        self.0
    }

    fn convert_to_type(s: &str) -> Option<Self::Output> {
        s.parse().ok().and_then(|n| T::try_from(n).ok())
    }

    fn convert_to_string(o: Self::Output) -> String {
        o.into().to_string()
    }
}

pub struct ParamsMapKey<T: QueryType>(StoredValue<InnerParamsMapKey<T>>)
where
    InnerParamsMapKey<T>: 'static;

impl<T: QueryType> Clone for ParamsMapKey<T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<T: QueryType> Copy for ParamsMapKey<T> {}

#[derive(Debug)]
pub struct InnerParamsMapKey<T: QueryType> {
    key: &'static str,
    default: T::Output,
    _maker: PhantomData<T>,
}

impl<T: QueryType> Clone for InnerParamsMapKey<T> {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            default: self.default.clone(),
            _maker: self._maker,
        }
    }
}

impl<T: QueryType> ParamsMapKey<T> {
    pub fn new(key: &'static str, default: T) -> Self {
        Self(store_value(InnerParamsMapKey {
            key,
            default: default.inner(),
            _maker: Default::default(),
        }))
    }

    pub fn key(&self) -> &'static str {
        self.0.with_value(|v| v.key)
    }
}

impl<T: QueryType> ParamsMapKey<T> {
    pub fn default(&self) -> T::Output {
        self.0.with_value(|v| v.default.clone())
    }
}

pub fn create_params_map_key<T: QueryType>(
    key: &'static str,
    default: T,
) -> ParamsMapKey<T> {
    ParamsMapKey::new(key, default)
}

pub trait MemoParamsMapExtra {
    /// generate new query string, should use with [`use_query_map`]
    fn with_query_map(
        &self,
        f: impl Fn(&mut ParamsMap) + 'static,
    ) -> Signal<String>;

    /// generate new url, should use with [`use_query_map`]
    fn with_query_map_url(
        &self,
        f: impl Fn(&mut ParamsMap) + 'static,
    ) -> Signal<String>;

    fn use_query<T: QueryType>(
        &self,
        query: ParamsMapKey<T>,
    ) -> Signal<Option<T::Output>>;

    fn use_query_with_default<T: QueryType>(
        &self,
        query: ParamsMapKey<T>,
    ) -> Signal<T::Output>;
}

impl MemoParamsMapExtra for Memo<ParamsMap> {
    fn with_query_map(
        &self,
        f: impl Fn(&mut ParamsMap) + 'static,
    ) -> Signal<String> {
        let map = *self;
        Signal::derive(move || {
            let mut map = map();
            f(&mut map);
            map.to_query_string()
        })
    }

    fn with_query_map_url(
        &self,
        f: impl Fn(&mut ParamsMap) + 'static,
    ) -> Signal<String> {
        let query = self.with_query_map(f);
        let location = use_location();
        let pathname = location.pathname;
        let hash = location.hash;
        Signal::derive(move || format!("{}{}{}", pathname(), hash(), query()))
    }

    fn use_query<T: QueryType>(
        &self,
        query: ParamsMapKey<T>,
    ) -> Signal<Option<T::Output>> {
        let map = *self;
        Signal::derive(move || map().get_query(query))
    }

    fn use_query_with_default<T: QueryType>(
        &self,
        query: ParamsMapKey<T>,
    ) -> Signal<T::Output> {
        let map = *self;
        Signal::derive(move || map().get_query_with_default(query))
    }
}

pub trait ParamsMapExtra {
    fn get_query<T>(&self, query: ParamsMapKey<T>) -> Option<T::Output>
    where
        T: QueryType;

    fn get_query_with_default<T>(&self, query: ParamsMapKey<T>) -> T::Output
    where
        T: QueryType;

    fn set_query<T>(
        &mut self,
        query: ParamsMapKey<T>,
        value: Option<T::Output>,
    ) where
        T: QueryType;

    fn to_url(&self) -> String;
}

impl ParamsMapExtra for ParamsMap {
    fn get_query<T>(&self, query: ParamsMapKey<T>) -> Option<T::Output>
    where
        T: QueryType,
    {
        self.get(query.key()).and_then(|v| T::convert_to_type(v))
    }

    fn get_query_with_default<T>(&self, query: ParamsMapKey<T>) -> T::Output
    where
        T: QueryType,
    {
        self.get_query(query)
            .unwrap_or_else(move || query.default())
    }

    fn set_query<T>(&mut self, query: ParamsMapKey<T>, value: Option<T::Output>)
    where
        T: QueryType,
    {
        let value = value.and_then(move |v| {
            query.0.with_value(|query| v != query.default).then_some(v)
        });
        match value {
            Some(value) => {
                self.insert(query.key().to_owned(), T::convert_to_string(value))
            }
            None => self.remove(query.key()),
        };
    }

    fn to_url(&self) -> String {
        let query = self.to_query_string();
        let location = use_location();
        let pathname = location.pathname.get_untracked();
        let hash = location.hash.get_untracked();
        format!("{pathname}{hash}{query}")
    }
}
