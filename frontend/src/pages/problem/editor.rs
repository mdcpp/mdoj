use leptos::*;
use leptos_query::*;
use leptos_router::{use_params, Params};
use tailwind_fuse::tw_join;

use crate::{components::*, utils::*};

#[derive(Params, Debug, PartialEq, Eq, Clone, Hash)]
struct EditorParams {
    id: i32,
}

async fn fetcher(token: Option<String>) -> Result<grpc::Languages> {
    let mut client = grpc::submit_client::SubmitClient::new(grpc::new_client());
    let langs = client.list_lang(().with_optional_token(token)).await?;
    Ok(langs.into_inner())
}

#[component]
pub fn ProblemEditor() -> impl IntoView {
    let params = use_params::<EditorParams>();
    let token = use_token();
    let scope = create_query(fetcher, Default::default());
    let query = scope.use_query(move || token());

    let editor = move || {
        query.data.get().map(|v| {
            v.map(|langs| {
                let id = params()?.id;
                Result::<_>::Ok(view! { <InnerProblemEditor id langs /> })
            })
        })
    };

    view! {
        <Suspense fallback=|| {
            view! { <p>loading</p> }
        }>
            <ErrorFallback>{editor}</ErrorFallback>
        </Suspense>
    }
}

#[component]
pub fn InnerProblemEditor(
    #[prop(into, optional)] class: String,
    id: i32,
    langs: grpc::Languages,
) -> impl IntoView {
    let options = langs
        .list
        .into_iter()
        .map(|lang| (lang.lang_ext, lang.lang_name.into_view()))
        .collect();
    let select_lang = create_rw_signal("".to_owned());
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
                        problem_id: id,
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
            select_lang(),
            editor_ref
                .with(|e| e.as_ref().map(|e| e.get_value()))
                .unwrap_or_default(),
        ));
    };

    let disabled = Signal::derive(move || select_lang.with(|v| v.is_empty()));

    view! {
        <form
            class=tw_join!("flex flex-col gap-4 h-full w-full bg-lighten p-3 rounded", class)
            on:submit=submit
        >
            <Editor lang_ext=select_lang editor_ref class="h-full" />
            <nav class="flex flex-row gap-4">
                <Select value=select_lang placeholder="Language" options></Select>
                <Button class="grow" type_="submit" disabled>
                    Submit
                </Button>
            </nav>
        </form>
    }
}
