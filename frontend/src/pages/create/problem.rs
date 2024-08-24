use leptos::*;
use leptos_router::*;

use crate::{components::*, utils::*};

#[component]
pub fn Problem() -> impl IntoView {
    let title = create_rw_signal("".to_owned());
    let difficulty = create_rw_signal(0u32);
    let time = create_rw_signal(0u64);
    let memory = create_rw_signal(0u64);
    let tags = create_rw_signal("".to_owned());
    let match_rule = create_rw_signal("".to_owned());
    let editor_ref = create_editor_ref();

    let token = use_token();
    let request_id = uuid::Uuid::new_v4().to_string();
    let create = create_action(
        move |(info, token): &(grpc::create_problem_request::Info, String)| {
            let mut client =
                grpc::problem_client::ProblemClient::new(grpc::new_client());
            let info = info.clone();
            let token = token.clone();
            let request_id = request_id.clone();
            let toast = use_toast();

            let navigate = use_navigate();
            async move {
                let id = client
                    .create(
                        grpc::CreateProblemRequest {
                            info,
                            request_id: Some(request_id),
                        }
                        .with_token(token),
                    )
                    .await?
                    .into_inner()
                    .id;

                navigate(&format!("/problem/{id}"), Default::default());
                toast(
                    ToastVariant::Success,
                    "Create problem success".into_view(),
                );
                Result::<_>::Ok(())
            }
        },
    );

    let submit = move |ev: ev::SubmitEvent| {
        ev.prevent_default();
        let info = grpc::create_problem_request::Info {
            title: title(),
            difficulty: difficulty(),
            time: time(),
            memory: memory(),
            tags: tags().split_whitespace().map(|s| s.into()).collect(),
            content: editor_ref.with(|e| {
                e.as_ref().map(|e| e.get_value()).unwrap_or_default()
            }),
            match_rule: match_rule
                .with(|rule| match rule.as_str() {
                    "EXACTLY" => grpc::MatchRule::MatchruleExactly,
                    "IGNORE_SNL" => grpc::MatchRule::MatchruleIgnoreSnl,
                    "SKIP_SNL" => grpc::MatchRule::MatchruleSkipSnl,
                    _ => unreachable!(),
                })
                .into(),
            // TODO: remove this when new API is complete
            order: 0.0,
        };
        create.dispatch((info, token().unwrap()));
    };

    let disabled = Signal::derive(move || {
        title.with(|v| v.is_empty()) || match_rule.with(|v| v.is_empty())
    });

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
                    <div class="flex flex-col min-w-fit flex-grow">
                        <label class="text-text pb-2">Difficulty</label>
                        <InputNumber value=difficulty />
                    </div>
                    <div class="flex flex-col min-w-fit flex-grow">
                        <label class="text-text pb-2">Time (nanosecond)</label>
                        <InputNumber value=time />
                    </div>
                    <div class="flex flex-col min-w-fit flex-grow">
                        <label class="text-text pb-2">Memory (byte)</label>
                        <InputNumber value=memory />
                    </div>
                    <div class="flex flex-col min-w-fit flex-grow">
                        <label class="text-text pb-2">Match Rule</label>
                        <Select value=match_rule placeholder="Match Rule">
                            <SelectOption value="EXACTLY">Exactly</SelectOption>
                            <SelectOption value="IGNORE_SNL">Ignore space and newline</SelectOption>
                            <SelectOption value="SKIP_SNL">Skip space and newline</SelectOption>
                        </Select>
                    </div>
                </div>

                <div class="w-full">
                    <label class="text-text pb-2">Content</label>
                    <Editor class="w-full h-full min-h-80" lang_ext="md" editor_ref />
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
