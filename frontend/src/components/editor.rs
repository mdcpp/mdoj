use gloo::console::console;
use leptos::*;

// #[derive(Debug, Clone, Copy)]
// pub enum Language {
//     Unselected,
//     Rust,
//     Python,
//     JavaScript,
//     C,
// }

// impl Language {
//     fn to_ident(self) -> Option<&'static str> {
//         match self {
//             Self::Unselected => None,
//             Self::Rust => Some("rust"),
//             Self::Python => Some("python"),
//             Self::JavaScript => Some("javascript"),
//             Self::C => Some("c"),
//         }
//     }
// }

pub fn get_editor_code() -> Option<String> {
    js_sys::eval("editor_inject_1083hdkjla.getValue()")
        .ok()?
        .as_string()
}

#[component]
pub fn Editor(
    #[prop(into, default = Some("javascript".to_string()).into())]
    language: MaybeSignal<Option<String>>,
    #[prop(into, optional)] on_submit: Option<Callback<String>>,
) -> impl IntoView {
    /// compacted version of the script
    /// ```javascript
    /// (async() => {
    ///     let parent = null;
    ///     while(parent==null || typeof monaco=='undefined'){
    ///         await new Promise(r=>setTimeout(r, 5));
    ///         parent = document.getElementById('editor-inject-1083hdkjla');
    ///     }
    ///
    ///     while(parent.firstChild) parent.removeChild(parent.firstChild);
    ///
    ///     let child = document.createElement('div');
    ///     parent.appendChild(child);
    ///     child.style = 'width:100%;height:65vh';
    ///   
    ///     monaco.editor.setTheme('vs-dark');
    ///     editor_inject_1083hdkjla = monaco.editor.create(child, {
    ///         value: '',
    ///         language: 'pro-lang'
    ///     });
    /// })()
    /// ```
    static EDITOR_SCRIPT_SOURCE: &str =
        "(async() => {let parent=null;while(parent==null || typeof \
         monaco=='undefined'){await new Promise(r=>setTimeout(r, \
         5));parent=document.getElementById('editor-inject-1083hdkjla');\
         }while(parent.firstChild) parent.removeChild(parent.firstChild);let \
         child=document.createElement('div');parent.appendChild(child);child.\
         style='width:100%;height:65vh';  \
         monaco.editor.setTheme('vs-dark');editor_inject_1083hdkjla=monaco.\
         editor.create(child, {value:'',language:'pro-lang'});})()";
    create_effect(move |_| {
        if let Some(ident) = language.get() {
            js_sys::eval(&EDITOR_SCRIPT_SOURCE.replace("pro-lang", &ident))
                .unwrap();
        }
    });

    view! {
        <div>
            <div id="editor-inject-1083hdkjla" style="width: 100%; border: 1px solid grey"></div>
        </div>
    }
}
