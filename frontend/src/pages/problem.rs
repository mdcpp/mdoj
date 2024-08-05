use leptos::*;
use leptos_router::*;
use leptos_use::*;

use crate::{
    components::*,
    errors::*,
    grpc::{self, WithToken},
    session::*,
};

#[derive(Params, PartialEq, Clone, Copy)]
struct ProblemParams {
    id: i32,
}

#[component]
pub fn Problem() -> impl IntoView {
    let params = use_params::<ProblemParams>();
    let token = use_token();

    let full_info = create_resource(
        move || (params(), token()),
        |(params, token)| {
            let mut problem_client =
                grpc::problem_client::ProblemClient::new(grpc::new_client());
            async move {
                let id: grpc::Id = params?.id.into();
                let full_info = problem_client
                    .full_info(id.with_optional_token(token))
                    .await?;
                Result::<_>::Ok(full_info.into_inner())
            }
        },
    );

    let langs = create_resource(
        move || token.get_untracked(),
        |token| {
            let mut submit_client =
                grpc::submit_client::SubmitClient::new(grpc::new_client());
            async move {
                let langs = submit_client
                    .list_lang(().with_optional_token(token))
                    .await?;
                Result::<_>::Ok(langs.into_inner())
            }
        },
    );

    let content = move || {
        full_info().map(|v| {
            v.map(|full_info| {
                view! { <ProblemContent full_info/> }
            })
        })
    };
    let editor = move || {
        langs().map(|v| {
            v.map(|langs| {
                let id: grpc::Id = params()?.id.into();

                Result::<_>::Ok(view! { <ProblemEditor id langs/> })
            })
        })
    };

    view! {
        <Suspense fallback=|| {
            view! { <p>loading</p> }
        }>
            <ErrorFallback>
                <main class="grow grid grid-cols-5 grid-flow-row gap-4 m-4">
                    {content} {editor}
                </main>
            </ErrorFallback>
        </Suspense>
    }
}

#[component]
fn ProblemContent(full_info: grpc::ProblemFullInfo) -> impl IntoView {
    let token = use_token();
    view! {
        <div
            class="h-full bg-lighten p-3 rounded"
            class=("col-span-3", is_some(token))
            class=("col-span-5", is_none(token))
        >
            <ul class="flex flex-row space-x-4 p-2 pt-0 mb-2 border-b-2 border-accent">
                <li>Problem</li>
                <li>Solution</li>
                <li>Discussion</li>
                <li>Submission</li>
            </ul>

            <h1 class="text-2xl my-2">{full_info.info.title}</h1>

            <Markdown content=full_info.content/>

            <hr class="border-t-2 border-accent mx-1"/>

            <ul class="flex flex-row justify-center space-x-4 p-1">
                <li>Memory : {full_info.memory}</li>
                <div class="h-auto border-l-2 border-accent"></div>
                <li>Time : {full_info.time}</li>
                <div class="h-auto border-l-2 border-accent"></div>
                <li>Difficulty : {full_info.difficulty}</li>
            </ul>
        </div>
    }
}

#[component]
fn ProblemEditor(id: grpc::Id, langs: grpc::Languages) -> impl IntoView {
    let select_option = langs.list.into_iter().map(|lang| {
            view! { <SelectOption value=lang.lang_ext>{lang.lang_name}</SelectOption> }
        }).collect_view();
    let select_lang = create_rw_signal("".to_owned());

    view! {
        <form
            class="flex flex-col h-full col-span-2 bg-lighten p-3 rounded"
            on:submit=move |e| {
                e.prevent_default();
            }
        >

            <ul class="flex flex-row justify-between p-2 pt-0 mb-2 border-b-2 border-accent">
                <li>Code</li>
                <li>
                    <Select value=select_lang placeholder="Language">
                        {select_option}
                    </Select>
                </li>
            </ul>
            <Editor lang_ext=select_lang class="h-full"/>
            <Button class="mt-auto" type_="submit">
                Submit
            </Button>
        </form>
    }
}
