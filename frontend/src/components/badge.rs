use leptos::*;
use tailwind_fuse::*;

use crate::utils::*;

#[component]
pub fn DifficultyBadge(difficulty: u32) -> impl IntoView {
    let style = match difficulty {
        0..500 => "border-green-500",
        500..1000 => "border-green-700",
        1000..1500 => "border-yellow-400",
        1500..2000 => "border-yellow-600",
        2000..2500 => "border-red-500",
        2500..3000 => "border-red-700",
        _ => "border-primary",
    };

    view! { <p class=tw_join!("p-1 m-1 w-min h-min border-2 rounded m-auto",style)>{difficulty}</p> }
}

#[component]
pub fn StateBadge(state: grpc::StateCode) -> impl IntoView {
    let (style, display) = match state {
        grpc::StateCode::Accepted => ("border-green-700", "AC"),
        grpc::StateCode::Unknown => ("border-yellow-400", "UNK"),
        grpc::StateCode::WrongAnswer => ("border-red-700", "WA"),
        grpc::StateCode::CompileError => ("border-yellow-600", "CE"),
        grpc::StateCode::RuntimeError => ("border-red-700", "RE"),
        grpc::StateCode::RestrictedFunction => ("border-yellow-500", "RF"),
        grpc::StateCode::TimeLimitExcess => ("border-red-500", "TLE"),
        grpc::StateCode::MemoryLimitExcess => ("border-red-500", "MLE"),
        grpc::StateCode::OutputLimitExcess => ("border-red-500", "OLE"),
    };
    view! { <p class=tw_join!("p-1 m-1 w-min h-min border-2 rounded m-auto",style)>{display}</p> }
}
