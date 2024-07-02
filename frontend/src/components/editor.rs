use gloo::console::console_dbg;
use leptos::*;
use leptos_dom::logging::console_log;
use leptos_meta::Script;

#[derive(Debug, Clone, Copy)]
pub enum Language {
    Unselected,
    Rust,
    Python,
    JavaScript,
    C,
}

impl Language {
    fn to_ident(self) -> Option<&'static str> {
        match self {
            Self::Unselected => None,
            Self::Rust => Some("rust"),
            Self::Python => Some("python"),
            Self::JavaScript => Some("javascript"),
            Self::C => Some("c"),
        }
    }
}

#[component]
pub fn Editor(
    #[prop(into, default = Language::JavaScript.into())] language: MaybeSignal<
        Language,
    >,
    #[prop(into, optional)] on_submit: Option<Callback<String>>,
) -> impl IntoView {
    /// compacted version of the script
    /// ```javascript
    /// function startup(){
    ///     setTimeout(()=>{
    ///         let parent=document.getElementById('editor-inject-1083hdkjla');
    ///         if (typeof monaco != 'undefined' && parent != null){
    ///             let child=document.createElement('div');
    ///             parent.appendChild(child);
    ///             child.style='width: 100%; height: 65vh';
    ///             monaco.editor.setTheme('vs-dark');
    ///             let editor = monaco.editor.create(child, {
    ///                 value: '',
    ///                 language: 'pro-lang'
    ///             });
    ///             console.log(editor.getValue())
    ///         }else{
    ///             startup();
    ///         }
    ///     }, 2);
    /// }
    /// startup();
    /// ```
    static EDITOR_SCRIPT_SOURCE: &str =
        "function startup(){setTimeout(()=>{let \
         parent=document.getElementById('editor-inject-1083hdkjla');if \
         (typeof monaco!='undefined'&&parent!=null){let \
         child=document.createElement('div');parent.appendChild(child);child.\
         style='width:100%;height:\
         65vh';monaco.editor.setTheme('vs-dark');let editor=\
         monaco.editor.create(child,{value:'',language:\
         'pro-lang'});console.log(editor.getValue())}else{startup();}},\
         2);}startup();";
    if let Some(ident) = language.get().to_ident() {
        create_effect(move |_| {
            js_sys::eval(&EDITOR_SCRIPT_SOURCE.replace("pro-lang", ident)).unwrap();
        });
    }

    #[cfg(not(feature = "ssr"))]
    if !js_sys::eval("typeof monaco")
        .unwrap()
        .as_string()
        .unwrap()
        .starts_with("undefined")
    {
        console_log("Monaco is already loaded");
        return view! {
            <div>
                <link
                    rel="stylesheet"
                    data-name="vs/editor/editor.main"
                    href="https://cdn.jsdelivr.net/npm/monaco-editor@0.50.0/min/vs/editor/editor.main.css"
                />
                <div
                    id="editor-inject-1083hdkjla"
                    style="width: 100%; border: 1px solid grey"
                ></div>
            </div>
        };
    }

    view! {
        <div>
            <link
                rel="stylesheet"
                data-name="vs/editor/editor.main"
                href="https://cdn.jsdelivr.net/npm/monaco-editor@0.50.0/min/vs/editor/editor.main.css"
            />
            <div id="editor-inject-1083hdkjla" style="width: 100%; border: 1px solid grey"></div>
            <Script>
                "var require = {paths:{vs:'https://cdn.jsdelivr.net/npm/monaco-editor@0.50.0/min/vs'}}"
            </Script>
            <Script src="https://cdn.jsdelivr.net/npm/monaco-editor@0.50.0/min/vs/loader.js"/>
            <Script src="https://cdn.jsdelivr.net/npm/monaco-editor@0.50.0/min/vs/editor/editor.main.nls.js"/>
            <Script src="https://cdn.jsdelivr.net/npm/monaco-editor@0.50.0/min/vs/editor/editor.main.js"/>
        </div>
    }
}
