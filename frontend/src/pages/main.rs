use crate::{components::*, config::*, pages::*};
use leptos::*;
use leptos_router::*;
use leptos_use::*;

#[component]
pub fn Main() -> impl IntoView {
    let (token, _) = use_token();
    view! {
        <div class="bg-background w-full h-full flex flex-col">
            <Router>
                <Navbar/>
                <main class="grow">
                    <Routes>
                        <Route path="" view=Home/>
                        <Route path="/problems" view=Problems/>
                        <Route path="/problem/:id" view=Problem/>
                        <Route path="/contests" view=Contests/>
                        <Route path="/about" view=About/>

                        <ProtectedRoute
                            path="/login"
                            redirect_path="/"
                            condition=is_none(token)
                            view=Login
                        />

                        <Route path="/*any" view=NotFound/>
                    </Routes>
                </main>
                <Footer/>
            </Router>
        </div>
    }
}