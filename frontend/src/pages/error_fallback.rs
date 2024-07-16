use leptos::*;

use crate::{error::ErrorKind, pages::*};

#[component]
pub fn ErrorFallback(children: Children) -> impl IntoView {
    view! { <ErrorBoundary fallback=fallback>{children()}</ErrorBoundary> }
}

fn fallback(errors: RwSignal<Errors>) -> impl IntoView {
    let fallback = move || {
        errors()
            .into_iter()
            .next()
            .map(|(_, err)| match err.into() {
                ErrorKind::NotFound => view! { <NotFound/> }.into_view(),
                ErrorKind::RateLimit => todo!(),
                ErrorKind::LoginRequire => todo!(),
                ErrorKind::OutOfRange => view! { <NotFound/> }.into_view(),
                ErrorKind::Network => todo!(),
                ErrorKind::ServerError(_) => {
                    view! { <InternalServerError/> }.into_view()
                }
                ErrorKind::PermissionDenied => todo!(),
                ErrorKind::Browser => todo!(),
            })
    };
    fallback.into_view()
}
