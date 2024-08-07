use leptos::*;
use leptos_router::*;

use super::{ProblemContent, ProblemEditor};
use crate::{
    errors::*,
    grpc::{self, WithToken},
    session::*,
};

#[derive(Params, PartialEq, Clone, Copy)]
struct ProblemParams {
    id: i32,
}

#[component(transparent)]
pub fn ProblemRouter() -> impl IntoView {
    view! {
        <Route path="problem/:id" view=Problem>
            <Route path="" view=Content/>
        </Route>
    }
}

#[component]
fn Problem() -> impl IntoView {
    let params = use_params::<ProblemParams>();
    let token = use_token();

    let langs = create_resource(
        move || token.get_untracked(),
        |token| {
            let mut client =
                grpc::submit_client::SubmitClient::new(grpc::new_client());
            async move {
                let langs =
                    client.list_lang(().with_optional_token(token)).await?;
                Result::<_>::Ok(langs.into_inner())
            }
        },
    );

    let editor = move || {
        langs().map(|v| {
            v.map(|langs| {
                let id = params()?.id;

                Result::<_>::Ok(
                    view! { <ProblemEditor id langs class="col-span-2 col-start-4"/> },
                )
            })
        })
    };

    view! {
        <main class="grow grid grid-cols-5 grid-flow-row gap-4">
            <Outlet/>
            <Suspense fallback=|| {
                view! { <p>loading</p> }
            }>
                <ErrorFallback>{editor}</ErrorFallback>
            </Suspense>
        </main>
    }
}

#[component]
fn Content() -> impl IntoView {
    let params = use_params::<ProblemParams>();
    let token = use_token();

    let full_info = create_resource(
        move || (params(), token()),
        |(params, token)| {
            let mut client =
                grpc::problem_client::ProblemClient::new(grpc::new_client());
            async move {
                let id: grpc::Id = params?.id.into();
                let full_info =
                    client.full_info(id.with_optional_token(token)).await?;
                Result::<_>::Ok(full_info.into_inner())
            }
        },
    );

    let content = move || {
        full_info().map(|v| {
            v.map(|full_info| {
                view! { <ProblemContent full_info class="col-span-3"/> }
            })
        })
    };
    view! {
        <Suspense fallback=|| {
            view! { <p>loading</p> }
        }>
            <ErrorFallback>{content}</ErrorFallback>
        </Suspense>
    }
}
