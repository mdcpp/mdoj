use leptos::*;
use leptos_router::*;
use leptos_use::*;
use problem::ProblemRouter;

use crate::{errors::NotFound, grpc, pages::*, session::*};

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

    view! {
        <Routes>
            <Route path="" view=Home/>
            <Route path="/problems" view=Problems/>
            <Route path="/submissions" view=Submission/>
            <Route path="/contests" view=Contests/>
            <Route path="/about" view=About/>
            <Route path="/rank" view=Rank/>
            <ProblemRouter/>

            <ProtectedRoute path="/login" redirect_path="/" condition=is_none(token) view=Login/>
            <ProtectedRoute
                path="/create/problem"
                redirect_path="/login"
                condition=can_create_problem_or_contest
                view=create::Problem
            />

            // Fallback
            <Route path="/*any" view=NotFound/>
        </Routes>
    }
}
