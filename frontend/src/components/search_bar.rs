use leptos::*;
use leptos_icons::*;
use tailwind_fuse::*;

use crate::components::*;

#[component]
pub fn SearchBar(
    #[prop(into, optional)] class: String,
    submit: impl Fn(ev::SubmitEvent, String) + 'static,
) -> impl IntoView {
    let search = create_rw_signal("".to_owned());
    view! {
        <form on:submit=move |e| submit(e, search.get_untracked()) class=tw_join!("relative",class)>
            <Input value=search class="grow"></Input>
            <button type="submit" class="absolute right-4 top-0 h-full">
                <Icon icon=icondata::BsSearch />
            </button>
        </form>
    }
}
