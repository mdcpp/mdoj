use leptos::{ev::MouseEvent, *};
use tailwind_fuse::*;

use super::Button;

#[derive(Debug, TwVariant)]
pub enum ModalLevel {
    #[tw(default, class = "")]
    Info,
    #[tw(class = "")]
    Warn,
    #[tw(class = "")]
    Error,
}

#[component]
pub fn Modal(
    level: ModalLevel,
    #[prop(into, optional)] on_close: Option<Callback<MouseEvent>>,
    children: Children,
) -> impl IntoView {
    // FIXME: what is level for? color?
    let _level = level;
    view! {
        <dialog open>
            {children()}
            <form method="dialog">

                {match on_close {
                    Some(f) => {
                        view! {
                            <Button type_="submit" on:click=f>
                                OK
                            </Button>
                        }
                    }
                    None => view! { <Button type_="submit">OK</Button> },
                }}

            </form>
        </dialog>
    }
}
