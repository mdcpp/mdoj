use leptos::{error::Error, *};

#[component]
pub fn CatchBoundary(children: Children) -> impl IntoView {
    let errors = create_rw_signal(Errors::default());
    provide_context(errors);

    children()
}

pub fn throw(error: impl Into<Error>) {
    let errors: Option<RwSignal<Errors>> = use_context();

    let Some(errors) = errors else {
        #[cfg(debug_assertions)]
        logging::debug_warn!(
            "Cannot find `CatchBoundary`/`ErrorBoundary` component, error will be ignore"
        );
        return;
    };
    errors().insert_with_default_key(error);
}

pub fn use_throw<E: Into<Error>>() -> impl Fn(E) {
    let errors: Option<RwSignal<Errors>> = use_context();

    let Some(errors) = errors else {
        logging::debug_warn!("Cannot find `CatchBoundary`/`ErrorBoundary` component");
        unreachable!();
    };
    move |error| errors().insert_with_default_key(error)
}

pub fn use_catch() -> RwSignal<Errors> {
    let errors: Option<RwSignal<Errors>> = use_context();

    let Some(errors) = errors else {
        logging::debug_warn!("Cannot find `CatchBoundary`/`ErrorBoundary` component");
        unreachable!();
    };
    errors
}
