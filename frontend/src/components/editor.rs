use std::{collections::HashMap, path};

use gloo::console::console;
use js_sys::{Object, Reflect};
use leptos::{leptos_dom::logging::console_log, *};
use wasm_bindgen::prelude::*;
use web_sys::Event;

#[wasm_bindgen]
extern "C" {
    fn require(modules: Vec<String>, on_load: &Closure<dyn FnMut()>);

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

    /// Create a new editor under `domElement`.
    /// `domElement` should be empty (not contain other dom nodes).
    /// The editor will read the size of `domElement`.
    #[wasm_bindgen(method, js_name = "create")]
    fn create_editor(
        this: &MonacoEditor,
        el: web_sys::HtmlElement,
        config: Option<Object>,
    );
}

#[component]
pub fn Editor(
    #[prop(into, default = "".to_owned().into())] language: MaybeSignal<String>,
    #[prop(into, default = "".to_owned())] class: String,
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
        Reflect::set(&*config, &"language".into(), &"rust".into()).unwrap();

        let init_monaco = Closure::once(move || {
            MONACO.editor().create_editor(
                (**(node_ref.get_untracked().unwrap())).clone(),
                Some(config),
            );
        });

        require(vec!["vs/editor/editor.main".into()], &init_monaco);
        init_monaco.forget();
    };
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

pub fn get_editor_code() -> Option<String> {
    js_sys::eval("editor_inject_1083hdkjla.getValue()")
        .ok()?
        .as_string()
}
