use leptos::*;

use crate::{
    components::*,
    grpc::{self, WithToken},
    session::*,
};

#[component]
pub fn Problem() -> impl IntoView {
    let title = create_rw_signal("".to_owned());
    let difficulty = create_rw_signal("".to_owned());
    let time = create_rw_signal("".to_owned());
    let memory = create_rw_signal("".to_owned());
    let token = use_token();
    // let mut client = grpc::problem_client::ProblemClient::new(grpc::new_client());
    // client.create(
    //     grpc::CreateProblemRequest {
    //         info: grpc::create_problem_request::Info {
    //             title: todo!(),
    //             difficulty: todo!(),
    //             time: todo!(),
    //             memory: todo!(),
    //             tags: todo!(),
    //             content: todo!(),
    //             match_rule: todo!(),
    //             order: todo!(),
    //         },
    //         request_id: todo!(),
    //     }
    //     .with_token(token().unwrap()),
    // );
    view! {
        <main class="grow flex items-center justify-center">
            <form
                class="flex flex-col flex-nowrap justify-center bg-slate-900 min-w-80 w-4/5 p-4"
                on:submit=move |e| {
                    e.prevent_default();
                }
            >

                <div class="flex flex-col">
                    <label class="text-text pb-2">Title</label>
                    <Input value=title/>
                </div>

                <div class="pt-4 flex flex-row flex-nowrap justify-between">
                    <div class="flex flex-col">
                        <label class="text-text pb-2">Difficulty</label>
                        <Input value=difficulty/>
                    </div>
                    <div class="flex flex-col pl-4">
                        <label class="text-text pb-2">Time (nanosecond)</label>
                        <Input value=time/>
                    </div>
                    <div class="flex flex-col pl-4">
                        <label class="text-text pb-2">Memory (byte)</label>
                        <Input value=title/>
                    </div>
                    <div class="flex flex-col pl-4">
                        <label class="text-text pb-2">Match Rule</label>
                        <Input value=title/>
                    </div>
                </div>

                <div class="pt-4 w-full">
                    <label class="text-text pb-2">Content</label>
                    <Editor class="w-full h-[60vh]"/>
                </div>
                <div class="pt-4 w-full">
                    <Button type_="submit" class="w-full">
                        Create
                    </Button>
                </div>
            </form>

        </main>
    }
}
