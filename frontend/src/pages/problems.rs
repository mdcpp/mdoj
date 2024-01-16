use crate::components::*;
use crate::grpc;
use crate::grpc::*;
use leptos::*;

#[component]
pub fn Problems() -> impl IntoView {
    // let client=ProblemSetClient::new(grpc::new_client().await?);
    // client.list(ListRequest{ size: todo!(), offset: todo!(), request: todo!() })
    view! { <h1>Problems</h1> }
}
