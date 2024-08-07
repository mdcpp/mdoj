use leptos::*;
use tailwind_fuse::tw_join;

use crate::{components::*, errors::*, grpc};

#[component]
pub fn ProblemEditor(
    #[prop(into, optional)] class: String,
    id: i32,
    langs: grpc::Languages,
) -> impl IntoView {
    let select_option = langs.list.into_iter().map(|lang| {
            view! { <SelectOption value=lang.lang_ext>{lang.lang_name}</SelectOption> }
        }).collect_view();
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
        <form class=tw_join!("flex flex-col h-full bg-lighten p-3 rounded", class) on:submit=submit>

            <ul class="flex flex-row justify-between p-2 pt-0 mb-2 border-b-2 border-accent">
                <li>Code</li>
                <li>
                    <Select value=select_lang placeholder="Language">
                        {select_option}
                    </Select>
                </li>
            </ul>
            <Editor lang_ext=select_lang editor_ref class="h-full"/>
            <Button class="mt-auto" type_="submit" disabled>
                Submit
            </Button>
        </form>
    }
}
