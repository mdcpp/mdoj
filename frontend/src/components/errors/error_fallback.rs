use leptos::*;

use super::*;
use crate::utils::*;

#[component]
pub fn ErrorFallback(
    #[prop(into, optional)] mut class: String,
    children: Children,
) -> impl IntoView {
    if class.is_empty() {
        class.push_str("contents");
    }
    let fallback = move |errors| {
        view! { <div class=class.clone()>{move || error_page(errors)}</div> }
    };
    view! { <ErrorBoundary fallback>{children()}</ErrorBoundary> }
}

fn error_page(errors: RwSignal<Errors>) -> impl IntoView {
    let fallback = move || {
        errors().into_iter().next().map(|(_, err)| {
            let err: Error = err.into();
            match err.kind {
                ErrorKind::NotFound => NotFound.into_view(),
                ErrorKind::RateLimit => todo!(),
                ErrorKind::Unauthenticated => todo!(),
                ErrorKind::OutOfRange => NotFound.into_view(),
                ErrorKind::Network => todo!(),
                ErrorKind::Internal => InternalServerError.into_view(),
                ErrorKind::PermissionDenied => todo!(),
                ErrorKind::Browser => todo!(),
                ErrorKind::MalformedUrl => todo!(),
                ErrorKind::ApiNotMatch => todo!(),
                ErrorKind::Unimplemented => Unimplemented.into_view(),
            }
        })
    };
    fallback.into_view()
}
