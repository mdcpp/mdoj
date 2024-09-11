use js_sys::{Promise, Uint8Array};
use leptos::*;
use leptos_router::*;
use uuid::Uuid;

use crate::{
    components::*,
    utils::{
        grpc::{
            create_problem_request as cp_req, create_testcase_request as ct_req,
        },
        *,
    },
};

async fn create(
    info: (Uuid, cp_req::Info),
    testcases: Vec<(Uuid, u32, Promise, Promise)>,
    token: String,
    toast: impl Fn(ToastVariant, View) + Copy + 'static,
    navigate: impl Fn(&str, NavigateOptions) + Clone + 'static,
) -> Result<()> {
    let mut client =
        grpc::problem_client::ProblemClient::new(grpc::new_client());
    let problem_id = client
        .create(
            grpc::CreateProblemRequest {
                info: info.1,
                request_id: Some(info.0.to_string()),
            }
            .with_token(token.clone()),
        )
        .await?
        .into_inner()
        .id;

    let mut client =
        grpc::testcase_client::TestcaseClient::new(grpc::new_client());

    for (req_id, score, input, output) in testcases {
        let input = Uint8Array::new(
            &wasm_bindgen_futures::JsFuture::from(input).await.unwrap(),
        )
        .to_vec();
        let output = Uint8Array::new(
            &wasm_bindgen_futures::JsFuture::from(output).await.unwrap(),
        )
        .to_vec();

        let testcase_id = client
            .create(
                grpc::CreateTestcaseRequest {
                    info: ct_req::Info {
                        score,
                        input,
                        output,
                    },
                    request_id: Some(req_id.to_string()),
                }
                .with_token(token.clone()),
            )
            .await?
            .into_inner()
            .id;
        client
            .add_to_problem(
                grpc::AddTestcaseToProblemRequest {
                    testcase_id,
                    problem_id,
                    request_id: None,
                }
                .with_token(token.clone()),
            )
            .await?;
    }

    navigate(&format!("/problem/{problem_id}"), Default::default());
    toast(ToastVariant::Success, "Create problem success".into_view());
    Result::<_>::Ok(())
}

#[component]
pub fn Problem() -> impl IntoView {
    let title = create_rw_signal("".to_owned());
    let difficulty = create_rw_signal(0u32);
    let time = create_rw_signal(0u64);
    let memory = create_rw_signal(0u64);
    let tags = create_rw_signal("".to_owned());
    let match_rule = create_rw_signal::<Option<grpc::MatchRule>>(None);
    let editor_ref = create_editor_ref();

    type TestcaseType = (Uuid, String, RwSignal<u32>, Promise, Promise);
    let testcases: RwSignal<Vec<TestcaseType>> = create_rw_signal(vec![]);

    let token = use_token();
    let request_id = Uuid::new_v4();
    let create = create_action(
        move |(info, token): &(grpc::create_problem_request::Info, String)| {
            let info = info.clone();
            let token = token.clone();
            let toast = use_toast();
            let navigate = use_navigate();
            let testcases = testcases.with(|list| {
                list.iter()
                    .map(|(req_id, _, score, input, output)| {
                        (*req_id, score.get(), input.clone(), output.clone())
                    })
                    .collect::<Vec<_>>()
            });

            create((request_id, info), testcases, token, toast, navigate)
        },
    );

    let submit = move |ev: ev::SubmitEvent| {
        ev.prevent_default();
        let info = grpc::create_problem_request::Info {
            title: title(),
            difficulty: difficulty(),
            time: time() * 1000,
            memory: memory() << 20,
            tags: tags().split_whitespace().map(|s| s.into()).collect(),
            content: editor_ref.with(|e| {
                e.as_ref().map(|e| e.get_value()).unwrap_or_default()
            }),
            match_rule: match_rule().unwrap().into(),
            // TODO: remove this when new API is complete
            order: 0.0,
        };
        create.dispatch((info, token().unwrap()));
    };

    let toast = use_toast();
    create_effect(move |_| {
        let Some(Err(err)) = create.value()() else {
            return;
        };
        toast(ToastVariant::Error, err.to_string().into_view());
    });

    let disabled = Signal::derive(move || {
        title.with(|v| v.is_empty())
            || create.pending()()
            || match_rule.with(|v| v.is_none())
    });

    let toast = use_toast();
    let toast_format_error = move |name| {
        toast(
            ToastVariant::Warning,
            view! {
                "The file "
                <code>{name}</code>
                " should in the format "
                <code>"`name`.`in`/`out`"</code>
            }
            .into_view(),
        );
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum Ty {
        In,
        Out,
    }
    let upload = move |files: Vec<web_sys::File>| {
        let mut file_infos: Vec<_> = files
            .into_iter()
            .filter_map(|file| {
                let full_name = file.name();
                let Some((name, ty)) = full_name.split_once(".") else {
                    toast_format_error(full_name);
                    return None;
                };
                let ty = match ty {
                    "in" => Ty::In,
                    "out" => Ty::Out,
                    _ => {
                        toast_format_error(full_name);
                        return None;
                    }
                };
                Some((name.to_owned(), ty, file.array_buffer()))
            })
            .collect();
        file_infos.sort_by_cached_key(|(name, ty, ..)| {
            (name.len(), name.clone(), *ty)
        });

        testcases.update(|testcase| {
            for infos in file_infos.windows(2) {
                let (a_name, a_ty, input) = &infos[0];
                let (b_name, b_ty, output) = &infos[1];
                if a_name != b_name || *a_ty != Ty::In || *b_ty != Ty::Out {
                    continue;
                }

                let score = create_rw_signal(0u32);
                testcase.push((
                    Uuid::new_v4(),
                    a_name.to_owned(),
                    score,
                    input.clone(),
                    output.clone(),
                ));
            }
        });
    };

    let list = view! {
        <For
            each=move || {
                testcases
                    .with(|testcase| {
                        testcase.iter().map(|v| (v.0, v.1.clone(), v.2)).collect::<Vec<_>>()
                    })
            }
            key=move |v| v.0
            children=move |v| view! { <Testcase name=v.1 score=v.2 /> }
        ></For>
    };

    let options = vec![
        (
            Some(grpc::MatchRule::MatchruleExactly),
            "Exactly".into_view(),
        ),
        (
            Some(grpc::MatchRule::MatchruleIgnoreSnl),
            "Ignore space and newline".into_view(),
        ),
        (
            Some(grpc::MatchRule::MatchruleSkipSnl),
            "Skip space and newline".into_view(),
        ),
    ];

    view! {
        <main class="container grow flex items-center justify-center py-10">
            <form class="flex flex-col flex-nowrap justify-center p-4 gap-4" on:submit=submit>
                <h1 class="text-xl">Create a new problem</h1>

                <div class="flex flex-col">
                    <label class="text-text pb-2">Title</label>
                    <Input value=title />
                </div>
                <div class="flex flex-col">
                    <label class="text-text pb-2">Tags</label>
                    <Input value=tags />
                </div>

                <div class="w-full flex-wrap flex flex-row flex-1 gap-4 justify-evenly">
                    <div class="flex flex-col min-w-fit grow">
                        <label class="text-text pb-2">Difficulty</label>
                        <InputNumber value=difficulty />
                    </div>
                    <div class="flex flex-col min-w-fit grow">
                        <label class="text-text pb-2">Time (MS)</label>
                        <InputNumber value=time />
                    </div>
                    <div class="flex flex-col min-w-fit grow">
                        <label class="text-text pb-2">Memory (MB)</label>
                        <InputNumber value=memory />
                    </div>
                    <div class="flex flex-col min-w-fit grow">
                        <label class="text-text pb-2">Match Rule</label>
                        <Select value=match_rule placeholder="Match Rule".into_view() options />
                    </div>
                </div>

                <div class="w-full">
                    <label class="text-text pb-2">Content</label>
                    <Editor class="w-full h-full min-h-80" lang_ext="md" editor_ref />
                </div>

                <div class="w-full">
                    <label class="text-text pb-2">Test Cases</label>
                    <InputFiles list upload class="gap-4"></InputFiles>
                </div>

                <div class="w-full">
                    <Button type_="submit" class="w-full" disabled>
                        Create
                    </Button>
                </div>
            </form>

        </main>
    }
}

#[component]
fn Testcase(#[prop(into)] name: String, score: RwSignal<u32>) -> impl IntoView {
    view! {
        <li class="w-full flex flex-row flex-1">
            <div class="flex flex-col min-w-fit grow">
                <label class="text-text pb-2">Name</label>
                <p>{name}</p>
            </div>
            <div class="flex flex-col min-w-fit grow">
                <label class="text-text pb-2">Score</label>
                <InputNumber value=score />
            </div>
        </li>
    }
}
