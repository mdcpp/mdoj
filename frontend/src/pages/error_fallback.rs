use leptos::*;

use crate::{error::ErrorKind, pages::*};

pub fn error_fallback(errors: RwSignal<Errors>) -> impl IntoView {
    let fallback = move || {
        errors()
            .into_iter()
            .next()
            .map(|(_, err)| match err.into() {
                ErrorKind::NotFound => view! { <NotFound/> }.into_view(),
                ErrorKind::RateLimit => todo!(),
                ErrorKind::LoginRequire => todo!(),
                ErrorKind::Network => todo!(),
                ErrorKind::ServerError(_) => todo!(),
                ErrorKind::PermissionDenied => todo!(),
                ErrorKind::Browser => todo!(),
            })
    };
    fallback.into_view()
}
