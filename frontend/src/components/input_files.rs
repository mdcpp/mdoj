use leptos::*;
use leptos_icons::*;
use leptos_use::*;
use tailwind_fuse::*;
use wasm_bindgen::JsCast;

use crate::components::*;
#[component]
pub fn InputFiles(
    #[prop(into)] list: View,
    #[prop(into, optional)] class: String,
    upload: impl Fn(Vec<web_sys::File>) + Clone + 'static,
) -> impl IntoView {
    let input = {
        let upload = upload.clone();
        move |e: ev::Event| {
            let select_files = js_sys::Array::from(
                &e.target()
                    .unwrap()
                    .unchecked_ref::<web_sys::HtmlInputElement>()
                    .files()
                    .unwrap(),
            )
            .into_iter()
            .map(web_sys::File::from)
            .collect();
            upload(select_files);
        }
    };

    let node_ref = create_node_ref::<html::Ul>();
    let UseDropZoneReturn {
        is_over_drop_zone, ..
    } = use_drop_zone_with_options(
        node_ref,
        UseDropZoneOptions::default().on_drop(move |e| {
            upload(e.files);
        }),
    );

    view! {
        <ul
            class=tw_join!("relative flex flex-col p-4 border-2 border-black-800",class)
            node_ref=node_ref
        >
            {list}
            <li class="bg-black-800">
                <label class="w-full flex flex-row gap-4 justify-center items-center cursor-pointer p-4">
                    <input type="file" multiple on:input=input class="hidden" />
                    <Icon icon=icondata::AiUploadOutlined class="size-6"></Icon>
                    "Select or Drop Files Here"
                </label>
            </li>
            <div class=move || {
                tw_join!(
                    "absolute size-full top-0 left-0 pointer-events-none flex flex-col text-text justify-center items-center bg-black-700 transition-opacity",
                    if is_over_drop_zone() { "opacity-100" } else { "opacity-0" }
                )
            }>
                <Icon icon=icondata::AiUploadOutlined class="size-1/2 max-h-full max-w-full"></Icon>
                <p>Drop Files Here</p>
            </div>
        </ul>
    }
}

#[component]
pub fn InputFilesItem(
    #[prop(into, optional)] disabled: MaybeSignal<bool>,
    #[prop(into)] name: String,
    delete: impl Fn(ev::MouseEvent) + 'static,
) -> impl IntoView {
    view! {
        <li
            class="flex flex-row disabled:pointer-events-none disabled:brightness-50"
            disabled=disabled
        >
            <p class="grow">{name}</p>
            <Button on:click=delete>
                <Icon icon=icondata::AiCloseOutlined />
            </Button>
        </li>
    }
}
