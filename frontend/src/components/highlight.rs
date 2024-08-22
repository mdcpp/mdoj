use leptos::*;
use syntect::{
    highlighting::ThemeSet, html::highlighted_html_for_string,
    parsing::SyntaxSet,
};

#[component]
pub fn Highlight() -> impl IntoView {
    let (value, set_value) = create_signal("".to_owned());
    let syn_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();
    let html = move || {
        logging::log!("updating");
        let syn = syn_set.find_syntax_by_extension("cpp").unwrap();
        let theme = &theme_set.themes["base16-ocean.dark"];
        highlighted_html_for_string(&value(), &syn_set, syn, theme).ok()
    };

    view! {
        <div class="relative h-full">
            <textarea
                class="opacity-0 absolute h-full w-full"
                contenteditable="true"
                spellcheck="false"
                on:input=move |e| {
                    logging::log!("change");
                    set_value(event_target_value(&e));
                }
            ></textarea>
            <div inner_html=html></div>
        </div>
    }
}
