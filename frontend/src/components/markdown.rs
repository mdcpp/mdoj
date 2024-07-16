use leptos::*;
use pulldown_cmark::{html::push_html, Parser};
use tailwind_fuse::tw_merge;

#[component]
pub fn Markdown(
    #[prop(into)] content: String,
    #[prop(into, default = "".into())] class: String,
) -> impl IntoView {
    let parser = Parser::new(&content);
    let mut html_buffer = String::new();
    push_html(&mut html_buffer, parser);
    view! { <div class=tw_merge!(class, "markdown rounded p-2") inner_html=html_buffer></div> }
}
