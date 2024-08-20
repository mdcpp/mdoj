use leptos::*;
use tailwind_fuse::tw_join;

use crate::{components::*, grpc};

#[component]
pub fn ProblemContent(
    #[prop(into, optional)] class: String,
    full_info: grpc::ProblemFullInfo,
) -> impl IntoView {
    view! {
        <div class=tw_join!("p-3 rounded h-full w-full flex flex-col", class)>
            <ul class="flex flex-row space-x-4 p-4 pt-0 mb-2 border-b-2 border-accent">
                <li>Problem</li>
                <li>Solution</li>
                <li>Discussion</li>
                <li>Submission</li>
            </ul>

            <h1 class="text-2xl my-2">{full_info.info.title}</h1>

            <div class="flex-grow relative overflow-y-auto bg-black-900">
                <Markdown content=full_info.content class="absolute h-full w-full top-0 left-0"/>
            </div>

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
