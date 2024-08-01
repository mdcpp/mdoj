use leptos::*;
use leptos_router::*;
use leptos_use::*;

use crate::{components::*, errors::NotFound, pages::*, session::use_token};

#[component]
pub fn Pages() -> impl IntoView {
    let token = use_token();

    view! {
        <Routes>
            <Route path="" view=Home/>
            <Route path="/problems" view=Problems/>
            <Route path="/submissions" view=Submission/>
            <Route path="/problem/:id" view=Problem/>
            <Route path="/contests" view=Contests/>
            <Route path="/about" view=About/>
            <Route path="/rank" view=Rank/>

            <ProtectedRoute path="/login" redirect_path="/" condition=is_none(token) view=Login/>
            <ProtectedRoute
                path="/create/problem"
                redirect_path="/login"
                condition=is_some(token)
                view=create::Problem
            />

            // Fallback
            <Route path="/*any" view=NotFound/>
        </Routes>
    }
}
