use leptos::*;
use leptos_icons::*;
use leptos_router::*;

use super::*;
use crate::{components::*, utils::*};

#[derive(Params, PartialEq, Clone, Copy)]
struct ProblemParams {
    id: i32,
}

#[component(transparent)]
pub fn ProblemRouter() -> impl IntoView {
    view! {
        <Route path="/problem/:id" view=Problem>
            <Route path="" view=Content ssr=SsrMode::Async />
            <Route path="/education" view=ProblemEducation ssr=SsrMode::Async />
            <Route path="/discussion" view=ProblemDiscussion />
            <Route path="/submission" view=ProblemSubmission />
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

                Result::<_>::Ok(view! { <ProblemEditor id langs /> })
            })
        })
    };

    view! {
        <main class="grow grid grid-cols-5 grid-flow-row gap-4">

            <div class="col-span-3 flex flex-row">
                <VerticalNavbar />
                <Outlet />
            </div>
            <div class="col-span-2 col-start-4">
                <Suspense fallback=|| {
                    view! { <p>loading</p> }
                }>
                    <ErrorFallback>{editor}</ErrorFallback>
                </Suspense>
            </div>
        </main>
    }
}

#[component]
fn VerticalNavbar() -> impl IntoView {
    view! {
        <ul class="grid auto-rows-min gap-y-8 p-2 my-auto bg-black-900">
            <VerticalNavbarButton icon=icondata::BsBook href="">
                Problem
            </VerticalNavbarButton>
            <VerticalNavbarButton icon=icondata::BiInstitutionSolid href="education">
                Education
            </VerticalNavbarButton>
            <VerticalNavbarButton icon=icondata::BsChatLeftText href="discussion">
                Discussion
            </VerticalNavbarButton>
            <VerticalNavbarButton icon=icondata::ImUpload href="submission">
                Submission
            </VerticalNavbarButton>
        </ul>
    }
}

#[component]
fn VerticalNavbarButton(
    icon: icondata::Icon,
    href: impl ToHref + 'static,
    children: Children,
) -> impl IntoView {
    view! {
        <li class="relative size-8 group">
            <A href=href>
                <Icon icon class="size-full group-hover:text-primary" />
            </A>
            <span class="absolute left-3/4 top-0 h-full px-4 ml-4 z-10 pointer-events-none bg-black-800 group-hover:opacity-100 group-hover:left-full transition-all opacity-0">
                {children()}
            </span>
        </li>
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
                view! { <ProblemContent full_info /> }
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
