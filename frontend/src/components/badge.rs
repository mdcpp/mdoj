use leptos::*;
use tailwind_fuse::*;

#[component]
pub fn Badge(difficulty: u32) -> impl IntoView {
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
