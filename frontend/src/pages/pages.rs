use leptos::*;
use leptos_router::*;
use leptos_use::*;

use super::{
    about::About, contest::Contest, contests::Contests, create, home::Home,
    login::Login, problem::ProblemRouter, problems::Problems, rank::Rank,
    submission::Submission,
};
use crate::{components::*, utils::*};

/// |Permission|Root|Admin|SuperUser|User|Guest
/// |:-|:-:|:-:|:-:|:-:|:-:|
/// |Register User|/|/|/|/|/|
/// |Join *any* Contest|V||||
/// |Create Admin|V||||
/// |Create SuperUser|V|V|||
/// |Create User|V|V|||
/// |Create Announcement for Contest|V|V|||
/// |Create/Publish Announcement|V|V|||
/// |Create/Publish Problem|V|V|V||
/// |Create/Publish Contest|V|V|V||
/// |Submit Problem|V|V|V|V|
/// > `/` 代表看 backend config
#[component]
pub fn Pages() -> impl IntoView {
    let token = use_token();
    let role = use_role();
    let can_create_problem_or_contest = move || {
        role().is_some_and(|role| match role {
            grpc::Role::User => false,
            grpc::Role::Super | grpc::Role::Admin | grpc::Role::Root => true,
        })
    };

    let show_footer = move || {
        !use_location()
            .pathname
            .with(|path| path.starts_with("/problem/"))
    };
    let page_wrapper = move || {
        view! {
            <Navbar />
            <Outlet />
            <Show when=show_footer fallback=|| ()>
                <Footer />
            </Show>
        }
    };

    view! {
        <Routes>
            <Route path="" view=page_wrapper>
                <Route path="" view=Home />
                <Route path="/problems" view=Problems ssr=SsrMode::Async />
                <Route path="/submissions" view=Submission />
                <Route path="/contests" view=Contests />
                <Route path="/contest" view=Contest />
                <Route path="/about" view=About />
                <Route path="/rank" view=Rank />
                <ProblemRouter />

                <ProtectedRoute
                    path="/login"
                    redirect_path="/"
                    condition=is_none(token)
                    view=Login
                />
                <ProtectedRoute
                    path="/create/problem"
                    redirect_path="/login"
                    condition=can_create_problem_or_contest
                    view=create::Problem
                />

                // Fallback
                <Route path="/*any" view=NotFound />
            </Route>
        </Routes>
    }
}
