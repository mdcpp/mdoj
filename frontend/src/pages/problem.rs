use leptos::*;
use leptos_router::*;
use leptos_use::*;
use uuid::Uuid;

use crate::{
    components::*,
    config::*,
    error::*,
    grpc::{self, problem_set_client, submit_set_client},
    pages::*,
};

#[derive(Params, PartialEq)]
struct ProblemParams {
    id: i32,
}

#[component]
pub fn Problem() -> impl IntoView {
    let params = use_params::<ProblemParams>();
    let id = move || {
        params.with(|params| {
            params
                .as_ref()
                .map(|v| v.id)
                .map_err(|_| ErrorKind::NotFound)
        })
    };
    let (token, _) = use_token();

    let problem_info = create_resource(id, |id| async move {
        let id = id?;
        let mut client = problem_set_client::ProblemSetClient::new(
            grpc::new_client().await?,
        );
        let resp = client.full_info(grpc::ProblemId { id }).await?.into_inner();
        Result::<_>::Ok(resp)
    });

    let submit_langs = create_resource(id, |id| async move {
        let mut client =
            submit_set_client::SubmitSetClient::new(grpc::new_client().await?);
        let resp = client.list_langs(()).await?.into_inner();
        Result::<_>::Ok(resp)
    });

    let submit = create_action(
        |(lang_uuid, code, id): &(String, String, Result<i32>)| {
            let lang_uuid = lang_uuid.clone();
            let code = code.clone();
            let id = id.clone();

            let (token, _) = use_token();
            let navigate = use_navigate();
            async move {
                let mut client = submit_set_client::SubmitSetClient::new(
                    grpc::new_client().await?,
                );
                let submit_id = client
                    .create(
                        grpc::CreateSubmitRequest {
                            lang: lang_uuid,
                            problem_id: grpc::ProblemId { id: id? },
                            code: code.into_bytes(),
                            request_id: Uuid::new_v4().simple().to_string(),
                        }
                        .with_token(token)?,
                    )
                    .await?
                    .into_inner();
                navigate(
                    &format!("/submit/{}", submit_id.id),
                    Default::default(),
                );
                Result::<_>::Ok(())
            }
        },
    );

    let select_lang = create_rw_signal("".to_owned());
    let code = create_rw_signal("".to_owned());

    let disabled =
        Signal::derive(move || select_lang().is_empty() || submit.pending()());

    view! {
        <Suspense fallback=|| {
            view! { <p>loading</p> }
        }>
            <ErrorBoundary fallback=error_fallback>
                <main class="grow grid grid-cols-5 grid-flow-row gap-4 m-4">
                    <div
                        class="h-full bg-lighten p-3 rounded"
                        class=("col-span-3", is_some(token))
                        class=("col-span-5", is_none(token))
                    >
                        <ul class="flex flex-row space-x-4 p-2 pt-0 mb-2 border-b-2 border-accent">
                            <li>Description</li>
                            <li>Solution</li>
                            <li>Discussion</li>
                            <li>Submission</li>
                        </ul>
                        {move || {
                            problem_info
                                .get()
                                .map(|info| {
                                    info.map(|info| {
                                        view! {
                                            <h1 class="text-2xl my-2">{info.info.title}</h1>

                                            <Markdown content=info.content/>

                                            <hr class="border-t-2 border-accent mx-1"/>

                                            <ul class="flex flex-row justify-center space-x-4 p-1">
                                                <li>Memory : {info.memory}</li>
                                                <div class="h-auto border-l-2 border-accent"></div>
                                                <li>Time : {info.time}</li>
                                                <div class="h-auto border-l-2 border-accent"></div>
                                                <li>Difficulty : {info.difficulty}</li>
                                            </ul>
                                        }
                                    })
                                })
                        }}

                    </div>
                    <form
                        class="flex flex-col h-full col-span-2 bg-lighten p-3 rounded"
                        class=("hidden", is_none(token))
                        on:submit=move |e| {
                            e.prevent_default();
                            submit.dispatch((select_lang(), code(), id()));
                        }
                    >

                        <ul class="flex flex-row justify-between p-2 pt-0 mb-2 border-b-2 border-accent">
                            <li>Code</li>
                            <li>
                                <Select value=select_lang placeholder="Language">
                                    {move || {
                                        submit_langs
                                            .get()
                                            .map(|langs| {
                                                langs
                                                    .map(|langs| {
                                                        langs
                                                            .list
                                                            .into_iter()
                                                            .map(|lang| {
                                                                view! {
                                                                    <SelectOption value=lang
                                                                        .lang_uid>{lang.lang_name}</SelectOption>
                                                                }
                                                            })
                                                            .collect_view()
                                                    })
                                            })
                                    }}

                                </Select>
                            </li>
                        </ul>
                        <textarea
                            class="w-full grow overflow-auto mb-4 bg-background outline-none"
                            on:input=move |e| code.set(event_target_value(&e))
                        ></textarea>
                        <Button class="mt-auto" kind="submit" disabled>
                            Submit
                        </Button>

                        // error report
                        {submit.value()}
                    </form>
                </main>
            </ErrorBoundary>
        </Suspense>
    }
}
