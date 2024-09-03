use leptos::*;
use leptos_query::*;
use leptos_router::{use_params, Params};
use tailwind_fuse::*;

use crate::{components::*, utils::*};

#[derive(Params, Debug, PartialEq, Eq, Clone, Hash)]
struct ContentParams {
    id: i32,
}

async fn fetcher(
    (params, token): (Result<ContentParams>, Option<String>),
) -> Result<grpc::ProblemFullInfo> {
    let mut client =
        grpc::problem_client::ProblemClient::new(grpc::new_client());
    let id: grpc::Id = params?.id.into();
    let full_info = client.full_info(id.with_optional_token(token)).await?;
    Result::<_>::Ok(full_info.into_inner())
}

#[component]
pub fn ProblemContent() -> impl IntoView {
    let params = use_params::<ContentParams>();
    let token = use_token();
    let scope = create_query(fetcher, Default::default());
    let result =
        scope.use_query(move || (params().map_err(|e| e.into()), token()));

    let content = move || {
        result.data.get().map(|v| {
            v.map(|full_info| {
                view! { <InnerProblemContent full_info /> }
            })
        })
    };
    view! {
        <Suspense fallback=|| {
            view! { <p>loading</p> }
        }>
            <ErrorFallback>{content}</ErrorFallback>
        </Suspense>
    }
}

#[component]
pub fn InnerProblemContent(
    #[prop(into, optional)] class: String,
    full_info: grpc::ProblemFullInfo,
) -> impl IntoView {
    view! {
        <div class=tw_join!("p-3 rounded h-full w-full flex flex-col", class)>
            <h1 class="text-2xl my-2">{full_info.info.title}</h1>

            <div class="flex-grow relative overflow-y-auto bg-black-900">
                <Markdown content=full_info.content class="absolute h-full w-full top-0 left-0" />
            </div>

            <hr class="border-t-2 border-accent mx-1" />

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
