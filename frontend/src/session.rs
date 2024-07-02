use leptos::*;
use leptos_use::{use_cookie_with_options, utils::JsonCodec, UseCookieOptions};
use serde::{Deserialize, Serialize};

use crate::grpc::Role;

/// Store user information
///
/// update only if user login/logout/refresh token
#[derive(Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct TokenInfo {
    pub token: String,
    pub role: Role,
}

/// Get token_info in cookie
///
/// set it value will cause update event,
pub fn use_token_info(
) -> (Signal<Option<TokenInfo>>, WriteSignal<Option<TokenInfo>>) {
    use_cookie_with_options::<_, JsonCodec>(
        "token_info",
        UseCookieOptions::default().max_age(60 * 60 * 1000),
    )
}

pub fn use_token() -> Signal<Option<String>> {
    let (user_info, _) = use_token_info();
    (move || user_info().as_ref().map(|s| s.token.clone())).into_signal()
}

pub fn use_role() -> Signal<Option<Role>> {
    let (user_info, _) = use_token_info();
    (move || user_info().as_ref().map(|s| s.role.clone())).into_signal()
}
