use leptos::*;
use leptos_router::*;

#[component]
pub fn Navbar() -> impl IntoView {
    view! {
        <nav class="bg-background sticky top-0 p-2 flex flex-row justify-between border-b-2 border-primary">
            <div class="flex flex-row flex-nowrap">
                <A href="/">
                    <img src="https://placehold.co/100" class="h-12 aspect-square mx-5"/>
                </A>
                <ul class="flex flex-row flex-nowrap justify-between items-center">
                    <li class="transition-opacity hover:opacity-60">
                        <A href="/problems" class="text-text text-base px-6">
                            Problems
                        </A>
                    </li>
                    <li class="transition-opacity duration-300 hover:opacity-60">
                        <A href="/contests" class="text-text text-base px-6">
                            Contests
                        </A>
                    </li>
                    <li class="transition-opacity duration-300 hover:opacity-60">
                        <A href="/submission" class="text-text text-base px-6">
                            Submission
                        </A>
                    </li>
                    <li class="transition-opacity duration-300 hover:opacity-60">
                        <A href="/rank" class="text-text text-base px-6">
                            Rank
                        </A>
                    </li>
                    <li class="transition-opacity duration-300 hover:opacity-60">
                        <A href="/about" class="text-text text-base px-6">
                            About
                        </A>
                    </li>
                </ul>
            </div>
            <div class="flex flex-row flex-nowrap justify-between items-center transition-opacity hover:opacity-60">
                <A href="/login" class="text-text text-base px-6">
                    Login
                </A>
            </div>
        </nav>
    }
}
