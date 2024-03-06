use leptos::*;
use pulldown_cmark::{html::push_html, Parser};

use super::Merge;

#[component]
pub fn Markdown(
    #[prop(into)] content: String,
    #[prop(into, optional)] class: Option<AttributeValue>,
) -> impl IntoView {
    let parser = Parser::new(&content);
    let mut html_buffer = String::new();
    push_html(&mut html_buffer, parser);
    view! { <div class=Merge(class, "markdown rounded p-2") inner_html=html_buffer></div> }
}
