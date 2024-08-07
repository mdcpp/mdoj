use leptos::*;

use super::{Error, ErrorKind, InternalServerError, NotFound};

#[component]
pub fn ErrorFallback(children: Children) -> impl IntoView {
    view! { <ErrorBoundary fallback=fallback>{children()}</ErrorBoundary> }
}

fn fallback(errors: RwSignal<Errors>) -> impl IntoView {
    let fallback = move || {
        errors().into_iter().next().map(|(_, err)| {
            let err: Error = err.into();
            match err.kind {
                ErrorKind::NotFound => view! { <NotFound/> }.into_view(),
                ErrorKind::RateLimit => todo!(),
                ErrorKind::Unauthenticated => todo!(),
                ErrorKind::OutOfRange => view! { <NotFound/> }.into_view(),
                ErrorKind::Network => todo!(),
                ErrorKind::Internal => {
                    view! { <InternalServerError/> }.into_view()
                }
                ErrorKind::PermissionDenied => todo!(),
                ErrorKind::Browser => todo!(),
                ErrorKind::MalformedUrl => todo!(),
            }
        })
    };
    fallback.into_view()
}
