use ::grpc::backend::Id;
use leptos::*;
use leptos_router::*;
use leptos_use::*;
use uuid::Uuid;

use crate::{
    components::{editor::get_editor_code, *},
    config::*,
    errors::*,
    grpc,
    pages::*,
    session::use_token,
};

#[derive(Params, PartialEq)]
struct ProblemParams {
    id: i32,
}

#[component]
pub fn Problem() -> impl IntoView {
    let params = use_params::<ProblemParams>();

    view! {
        // <Suspense fallback=|| {
        //     view! { <p>loading</p> }
        // }>
        //     <ErrorFallback>
        //         <main class="grow grid grid-cols-5 grid-flow-row gap-4 m-4">
        //             <div
        //                 class="h-full bg-lighten p-3 rounded"
        //                 class=("col-span-3", is_some(token))
        //                 class=("col-span-5", is_none(token))
        //             >
        //                 <ul class="flex flex-row space-x-4 p-2 pt-0 mb-2 border-b-2 border-accent">
        //                     <li>Description</li>
        //                     <li>Solution</li>
        //                     <li>Discussion</li>
        //                     <li>Submission</li>
        //                 </ul>
        //                 {move || {
        //                     problem_info
        //                         .get()
        //                         .map(|info| {
        //                             info.map(|info| {
        //                                 view! {
        //                                     <h1 class="text-2xl my-2">{info.info.title}</h1>

        //                                     <Markdown content=info.content/>

        //                                     <hr class="border-t-2 border-accent mx-1"/>

        //                                     <ul class="flex flex-row justify-center space-x-4 p-1">
        //                                         <li>Memory : {info.memory}</li>
        //                                         <div class="h-auto border-l-2 border-accent"></div>
        //                                         <li>Time : {info.time}</li>
        //                                         <div class="h-auto border-l-2 border-accent"></div>
        //                                         <li>Difficulty : {info.difficulty}</li>
        //                                     </ul>
        //                                 }
        //                             })
        //                         })
        //                 }}

        //             </div>
        //             <form
        //                 class="flex flex-col h-full col-span-2 bg-lighten p-3 rounded"
        //                 class=("hidden", is_none(token))
        //                 on:submit=move |e| {
        //                     e.prevent_default();
        //                     submit.dispatch((select_lang(), id()));
        //                 }
        //             >

        //                 <ul class="flex flex-row justify-between p-2 pt-0 mb-2 border-b-2 border-accent">
        //                     <li>Code</li>
        //                     <li>
        //                         <Select value=select_lang placeholder="Language">
        //                             {move || {
        //                                 submit_langs
        //                                     .get()
        //                                     .map(|langs| {
        //                                         langs
        //                                             .map(|langs| {
        //                                                 langs
        //                                                     .list
        //                                                     .into_iter()
        //                                                     .map(|lang| {
        //                                                         view! {
        //                                                             <SelectOption value=lang
        //                                                                 .lang_uid>{lang.lang_name}</SelectOption>
        //                                                         }
        //                                                     })
        //                                                     .collect_view()
        //                                             })
        //                                     })
        //                             }}

        //                         </Select>
        //                     </li>
        //                 </ul>
        //                 <Editor language=select_lang/>
        //                 <Button class="mt-auto" kind="submit" disabled>
        //                     Submit
        //                 </Button>

        //                 // error report
        //                 {submit.value()}
        //             </form>
        //         </main>
        //     </ErrorFallback>
        // </Suspense>
    }
}
