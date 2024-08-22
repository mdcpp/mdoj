use leptos::*;
use leptos_query::*;

use super::session::*;

pub fn provide_query_service() {
    provide_query_client();
    let token = use_token();
    let client = use_query_client();
    create_effect(move |_| {
        token.with(|_| client.invalidate_all_queries());
    });
}
