mod content;
mod discussion;
mod editor;
mod education;
mod submission;
pub use content::ProblemContent;
pub use discussion::ProblemDiscussion;
pub use editor::ProblemEditor;
pub use education::ProblemEducation;
use leptos::*;
use leptos_icons::*;
use leptos_router::*;
pub use submission::ProblemSubmission;

use crate::{components::*, utils::*};

#[component(transparent)]
pub fn ProblemRouter() -> impl IntoView {
    view! {
        <Route path="/problem/:id" view=Problem>
            <Route path="" view=ProblemContent ssr=SsrMode::Async />
            <Route path="/education" view=ProblemEducation ssr=SsrMode::Async />
            <Route path="/discussion" view=ProblemDiscussion />
            <Route path="/submission" view=ProblemSubmission />
        </Route>
    }
}

#[derive(Params, Debug, PartialEq, Eq, Clone, Hash)]
struct ProblemParams {
    id: i32,
}

#[component]
fn Problem() -> impl IntoView {
    let params_error = use_params::<ProblemParams>()().map(|_| ());
    view! {
        <main class="grow grid grid-cols-5 grid-flow-row gap-4">
            <ErrorFallback>
                <div class="col-span-3 flex flex-row">
                    <VerticalNavbar />
                    <Outlet />
                </div>
                <div class="col-span-2 col-start-4">
                    <ProblemEditor />
                </div>
                // error report
                {params_error}
            </ErrorFallback>
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
