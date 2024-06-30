use leptos::{ev::MouseEvent, *};

use super::Button;

pub enum ModalLevel {
    Info,
    Warn,
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
                            <Button kind="submit" on_click=f>
                                OK
                            </Button>
                        }
                    }
                    None => view! { <Button kind="submit">OK</Button> },
                }}

            </form>
        </dialog>
    }
}
