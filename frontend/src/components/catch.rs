use std::fmt::Debug;

use leptos::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
struct Ball<T>(Option<T>);

#[component]
pub fn CatchBoundary<T: 'static>(
    children: Children,
    #[prop(optional)] ball: Option<T>,
) -> impl IntoView {
    provide_catch(ball);

    children()
}

/// provide catch context
pub fn provide_catch<T: 'static>(ball: Option<T>) {
    let ball = create_rw_signal(Ball(ball));
    provide_context(ball);
}

/// `throw(...), catch signal, destroy_ball(...)`
pub fn use_ball<T: 'static + Clone + Debug>(
) -> (impl Fn(T), Signal<Option<T>>, impl Fn()) {
    let ctx_ball: RwSignal<Ball<T>> = expect_context();

    (
        move |ball| ctx_ball.set(Ball(Some(ball))),
        Signal::derive(move || ctx_ball.get().0),
        move || ctx_ball.set(Ball(None)),
    )
}

/// check has ball in `CatchBoundary`
pub fn use_has_ball<T: 'static + Clone>() -> Signal<bool> {
    let ctx_ball: RwSignal<Ball<T>> = expect_context();

    Signal::derive(move || ctx_ball.get().0.is_some())
}
