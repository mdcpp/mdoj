use leptos::*;
use leptos_router::*;

use crate::{
    components::*,
    config::{use_token, WithToken},
    error::*,
    grpc::{problem_set_client::*, *}, pages::problems::toggle::Toggle,
};


#[derive(Clone, PartialEq, Default)]
enum Endpoint {
    #[default]List=1,
    ListBy=2,
    Text=3
}

impl IntoParam for Endpoint{
    fn into_param(value: Option<&str>, name: &str)-> Result<Self, ParamsError> {
        Ok(match value.unwrap_or_default(){
            "1"=>Endpoint::List,
            "2"=>Endpoint::ListBy,
            _=>Endpoint::Text,
        })
    }
}

#[derive(Default, Clone, PartialEq, Params)]
struct Page {
    pager: Option<String>,
    offset: Option<usize>,
    endpoints: Endpoint
}

fn difficulty_color(difficulty: u32) -> impl IntoView {
    let color:&'static str =match difficulty {
        0..=1000 => "green",
        1001..=1500 => "orange",
        _ => "red",
    };
    view! {
        <span class=format!("bg-{} text-{} text-xs font-medium me-2 px-2.5 py-0.5 rounded border border-{}", color, color, color)>
            {difficulty}
        </span>
    }
}

#[component]
pub fn ProblemSearch() -> impl IntoView{
    // 1. Make it works
    // 2. Make it pretty
    // 3. Integrate with the problem list
    let search_text = create_rw_signal("".to_owned());
    let reverse = create_rw_signal(false);

    let submit=create_action(move |(search_text, reverse): &(String, bool)| {
        let serach_text = search_text.clone();

        let navigate = use_navigate();
        let (get_token, _) = use_token();

        async move {
            // let mut problem_set = problem_set_client::ProblemSetClient::new(
            //     new_client().await?,
            // );
            // match search_text.is_empty(){
            //     true=>{
            //         let resp = problem_set
            //             .list(
            //                 ListProblemRequest {
            //                     size: 50,
            //                     offset: None,
            //                     request: Some(
            //                         list_problem_request::Request::Create(
            //                             list_problem_request::Create {
            //                                 sort_by: ProblemSortBy::UpdateDate
            //                                     .into(),
            //                                 start_from_end: Some(*reverse),
            //                             },
            //                         ),
            //                     ),
            //                 }
            //                 .with_token(get_token()),
            //             )
            //             .await?;
            //         let resp = resp.into_inner();
            //         Some(resp)
            //     }
            // }
            todo!()
        }
    });

    let disabled=Signal::derive(move || {
        submit.pending()()
    });

    view!{
        <div>
            <label for="search_text" class="text-text pb-2">
                List of problems
            </label>
            <TextInput 
                id="search_text"
                value=search_text
                placeholder="Title tag1,tag2"
            />

            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <Button kind="submit" class="w-full" disabled>
                    Search
                </Button>
                <Toggle value=reverse /><span>Start from end</span>
            </div>
        </div>
    }
}

#[component]
pub fn Problems() -> impl IntoView {
    let params = use_params::<Page>();
    let page = move || params.with(|v| v.clone().unwrap_or_default());
    let (token, _) = use_token();
    let page_and_token = move || (page(), token());

    let problems =
        create_resource(page_and_token, |(page, token)| async move {
            let result: Result<ListProblemResponse> = async {
                Ok(ProblemSetClient::new(new_client().await?)
                    .list(
                        ListProblemRequest {
                            size: 50,
                            offset: None,
                            request: Some(
                                list_problem_request::Request::Create(
                                    list_problem_request::Create {
                                        sort_by: ProblemSortBy::UpdateDate
                                            .into(),
                                        start_from_end: Some(false),
                                    },
                                ),
                            ),
                        }
                        .with_token(token),
                    )
                    .await?
                    .into_inner())
            }
            .await;
            match result {
                Ok(v) => Some(v),
                Err(e) => None,
            }
        });

    view! {
        <div class="h-full container container-lg items-center justify-between text-lg">
            <ProblemSearch/>
            <Transition fallback=move || {
                view! { <p>Loading</p> }
            }>
                <div class="table w-full table-auto">
                    <div class="table-header-group text-left">
                        <div class="table-row">
                            <div class="table-cell">Title</div>
                            <div class="hidden md:table-cell">AC Rate</div>
                            <div class="hidden md:table-cell">Attempt</div>
                            <div class="table-cell">Difficulty</div>
                        </div>
                    </div>
                    <div class="table-row-group" style="line-height: 3rem">
                        {move || {
                            problems
                                .get()
                                .map(|v| {
                                    v.map(|v| {
                                        view! {
                                            {v
                                                .list
                                                .into_iter()
                                                .map(|info| {
                                                    view! {
                                                        <div class="table-row odd:bg-gray">
                                                            <div class="w-2/3 truncate table-cell">
                                                                <A href=format!("/problem/{}", info.id.id)>{info.title}</A>
                                                            </div>
                                                            <div class="text-center hidden md:table-cell">{info.ac_rate} %</div>
                                                            <div class="text-center hidden md:table-cell">{info.submit_count}</div>
                                                            <div class="table-cell">{difficulty_color(info.difficulty)}</div>
                                                        </div>
                                                    }
                                                })
                                                .collect_view()}
                                        }
                                            .into_view()
                                    })
                                })
                        }}
                    </div>
                </div>
                <ul>
                    <li>-1</li>
                    <li>0</li>
                    <li>+1</li>
                </ul>
            </Transition>
        </div>
    }
}
