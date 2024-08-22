use js_sys::{Function, Object, Reflect};
use leptos::*;
use wasm_bindgen::prelude::*;
use web_sys::Event;

use crate::utils::*;

#[wasm_bindgen]
extern "C" {
    fn require(modules: Vec<String>, on_load: &Function);

    #[wasm_bindgen(js_namespace = require, js_name="config")]
    fn loader_config(config: Object);
}

#[wasm_bindgen]
extern "C" {
    type Monaco;

    #[wasm_bindgen(js_name = "monaco")]
    static MONACO: Monaco;

    type MonacoEditor;

    #[wasm_bindgen(method, getter)]
    fn editor(this: &Monaco) -> MonacoEditor;

    pub type Editor;

    /// Create a new editor under `el`.
    /// `el` should be empty (not contain other dom nodes).
    /// The editor will read the size of `el`.
    #[wasm_bindgen(method, js_name = "create")]
    fn create_editor(
        this: &MonacoEditor,
        el: web_sys::HtmlElement,
        config: Option<Object>,
    ) -> Editor;

    type ITextModel;

    #[wasm_bindgen(method, js_name = "setModelLanguage")]
    fn set_model_language(
        this: &MonacoEditor,
        model: ITextModel,
        mime_type_or_language_id: String,
    );

    #[wasm_bindgen(method, js_name = "getValue")]
    pub fn get_value(this: &Editor) -> String;

    #[wasm_bindgen(method, js_name = "getModel")]
    pub fn get_model(this: &Editor) -> Option<ITextModel>;

}

pub fn create_editor_ref() -> RwSignal<Option<Editor>> {
    create_rw_signal(None)
}

#[component]
pub fn Editor(
    #[prop(into, optional)] lang_ext: MaybeSignal<String>,
    #[prop(into, optional)] class: String,
    #[prop(optional)] editor_ref: RwSignal<Option<Editor>>,
) -> impl IntoView {
    let node_ref = create_node_ref::<html::Div>();
    let on_load = move |_| {
        let c = Object::new();
        let paths = Object::new();
        Reflect::set(
            &*paths,
            &"vs".into(),
            &"https://cdn.jsdelivr.net/npm/monaco-editor@0.50.0/min/vs".into(),
        )
        .unwrap();
        Reflect::set(&*c, &"paths".into(), &*paths).unwrap();
        loader_config(c);

        let config = Object::new();
        Reflect::set(&*config, &"theme".into(), &"vs-dark".into()).unwrap();
        Reflect::set(&*config, &"automaticLayout".into(), &true.into())
            .unwrap();

        let init_monaco = Closure::once_into_js(move || {
            let editor = MONACO.editor().create_editor(
                (**(node_ref.get_untracked().unwrap())).clone(),
                Some(config),
            );
            editor_ref.set(Some(editor))
        });

        require(
            vec!["vs/editor/editor.main".into()],
            init_monaco.unchecked_ref(),
        );
    };

    create_effect(move |_| {
        editor_ref.with(|editor| {
            let Some(model) = editor.as_ref().map(|e| e.get_model()).flatten()
            else {
                return;
            };

            let lang_ext = lang_ext();
            let Some(lang) = frontend_config()
                .extension_language_mappings
                .iter()
                .find(|lang| lang.extension.contains(&lang_ext))
                .map(|lang| lang.language.clone())
            else {
                return;
            };

            MONACO.editor().set_model_language(model, lang);
        });
    });

    view! {
        <div node_ref=node_ref class=class></div>
        <DynamicLoad
            src="https://cdn.jsdelivr.net/npm/monaco-editor@0.50.0/min/vs/loader.js"
            on_load
        />
    }
}

#[component]
fn DynamicLoad(
    #[prop(into)] src: String,
    on_load: impl FnMut(Event) + 'static,
) -> impl IntoView {
    let (load_src, set_load_src) = create_signal(None);
    let node_ref = create_node_ref::<html::Script>();
    node_ref.on_load(move |_| {
        set_load_src(Some(src));
    });
    view! { <script on:load=on_load src=load_src node_ref=node_ref></script> }
}
