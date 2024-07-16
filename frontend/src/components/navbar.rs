use leptos::*;
use leptos_router::*;
use leptos_use::*;

use crate::session::use_token;

#[component]
pub fn Navbar() -> impl IntoView {
    let token = use_token();
    view! {
        <nav class="bg-slate-900 sticky top-0 p-2 flex flex-row justify-between border-b-2 border-secondary z-10">
            <div class="flex flex-row flex-nowrap">
                <A href="/">
                    <img src="https://placehold.co/100" class="h-12 aspect-square mx-5"/>
                </A>
                <ul class="flex flex-row flex-nowrap justify-between items-center text-base">
                    <NavbarLink href="/problems">Problems</NavbarLink>
                    <NavbarLink href="/contests">Contests</NavbarLink>
                    <NavbarLink href="/submissions">Submission</NavbarLink>
                    <NavbarLink href="/rank">Rank</NavbarLink>
                    <NavbarLink href="/about">About</NavbarLink>
                </ul>
            </div>
            <div class="flex flex-row flex-nowrap justify-between items-center transition-opacity hover:opacity-60">
                <Show
                    when=is_some(token)
                    fallback=move || {
                        view! {
                            <A href="/login" class="text-text text-base px-6">
                                Login
                            </A>
                        }
                    }
                >

                    <img src="https://placehold.co/100" class="h-12 aspect-square mx-5"/>
                </Show>
            </div>
        </nav>
    }
}

#[component]
fn NavbarLink(
    href: impl ToHref + 'static,
    children: Children,
) -> impl IntoView {
    view! {
        <li class="transition-opacity duration-300 hover:opacity-60">
            <A href class="px-6">
                {children()}
            </A>
        </li>
    }
}
