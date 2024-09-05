use leptos::*;
use leptos_query::*;
use leptos_router::{use_params, Params};

use crate::{components::*, utils::*};

#[derive(Params, Debug, PartialEq, Eq, Clone, Hash, Default)]
struct EditorParams {
    id: i32,
}

#[component]
pub fn ProblemEditor() -> impl IntoView {
    let params = use_params::<EditorParams>();

    let select_lang = create_rw_signal::<Option<grpc::Language>>(None);
    let editor_ref = create_editor_ref();

    let submit_problem =
        create_action(move |(lang_uid, code): &(String, String)| {
            let lang_uid = lang_uid.clone();
            let code = code.as_bytes().to_vec();

            let mut client =
                grpc::submit_client::SubmitClient::new(grpc::new_client());
            async move {
                let id = client
                    .create(grpc::CreateSubmitRequest {
                        lang_uid,
                        problem_id: params.get_untracked()?.id,
                        code,
                        request_id: None,
                    })
                    .await?;
                Result::<_>::Ok(id.into_inner().id)
            }
        });

    let submit = move |e: ev::SubmitEvent| {
        e.prevent_default();
        submit_problem.dispatch((
            select_lang.with(|v| v.as_ref().unwrap().lang_uid.clone()),
            editor_ref
                .with(|e| e.as_ref().map(|e| e.get_value()))
                .unwrap_or_default(),
        ));
    };

    let disabled = Signal::derive(move || select_lang.with(|v| v.is_none()));

    view! {
        <form class="flex flex-col gap-4 h-full w-full bg-lighten p-3 rounded" on:submit=submit>
            <Editor
                lang_ext=(move || {
                    select_lang.with(|v| v.as_ref().map(|v| v.lang_ext.clone()).unwrap_or_default())
                })
                    .into_signal()
                editor_ref
                class="h-full"
            />
            <nav class="flex flex-row gap-4">
                <LangSelect select_lang />
                <Button class="grow" type_="submit" disabled>
                    Submit
                </Button>
            </nav>
        </form>
    }
}

async fn fetcher(token: Option<String>) -> Result<grpc::Languages> {
    let mut client = grpc::submit_client::SubmitClient::new(grpc::new_client());
    let langs = client.list_lang(().with_optional_token(token)).await?;
    Ok(langs.into_inner())
}

#[component]
fn LangSelect(select_lang: RwSignal<Option<grpc::Language>>) -> impl IntoView {
    let scope = create_query(fetcher, Default::default());
    let token = use_token();
    let query = scope.use_query(token);

    let select = move || {
        query.data.get().map(|v|v.map(|v|{
            let options = v
                .list
                .into_iter()
                .map(|lang| {
                    let option=lang.lang_name.clone().into_view();
                    (
                        Some(lang),
                        option,
                    )
                })
                .collect();

            view! { <Select value=select_lang placeholder="Language".into_view() options></Select> }
        }))
    };
    view! {
        <Suspense fallback=|| {
            view! { <p>loading</p> }
        }>
            <ErrorFallback>{select}</ErrorFallback>
        </Suspense>
    }
}
