pub mod button;
pub mod catch;
pub mod footer;
pub mod highlight;
pub mod markdown;
pub mod modal;
pub mod navbar;
pub mod redirect_if;
pub mod select;
pub mod text_input;
use std::rc::Rc;

pub use button::Button;
pub use catch::{provide_catch, use_ball, use_has_ball, CatchBoundary};
pub use footer::Footer;
pub use highlight::Highlight;
use leptos::*;
pub use markdown::Markdown;
pub use modal::{Modal, ModalLevel};
pub use navbar::Navbar;
pub use redirect_if::RedirectIf;
pub use select::{Select, SelectOption};
pub use text_input::TextInput;

/// Merge 2 attribute into 1
pub struct Merge<A: IntoAttribute, B: IntoAttribute>(A, B);

impl<A: IntoAttribute, B: IntoAttribute> IntoAttribute for Merge<A, B> {
    fn into_attribute(self) -> Attribute {
        let a = self.0.into_attribute();
        let b = self.1.into_attribute();
        let func = move || {
            format!(
                "{} {}",
                a.as_nameless_value_string().unwrap_or_default(),
                b.as_nameless_value_string().unwrap_or_default()
            )
            .into_attribute()
        };
        Attribute::Fn(Rc::new(func))
    }

    fn into_attribute_boxed(self: Box<Self>) -> Attribute {
        self.into_attribute()
    }
}
