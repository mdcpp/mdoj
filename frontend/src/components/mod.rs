pub mod button;
pub mod footer;
pub mod navbar;
pub mod text_input;

use std::rc::Rc;

pub use button::Button;
pub use footer::Footer;
pub use navbar::Navbar;
pub use text_input::TextInput;

use leptos::*;

/// A Optional generic function
pub struct Caller<I>(Option<Box<dyn Fn(I)>>);

impl<I> Caller<I> {
    pub fn call(&mut self, input: I) {
        if let Some(func) = &mut self.0 {
            func.as_mut()(input);
        }
    }
}

impl<F, I> From<F> for Caller<I>
where
    F: Fn(I) + 'static,
{
    fn from(func: F) -> Self {
        Self(Some(Box::from(func)))
    }
}

impl<I> Default for Caller<I> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<I> AsRef<Option<Box<dyn Fn(I)>>> for Caller<I> {
    fn as_ref(&self) -> &Option<Box<dyn Fn(I)>> {
        &self.0
    }
}

impl<I> AsMut<Option<Box<dyn Fn(I)>>> for Caller<I> {
    fn as_mut(&mut self) -> &mut Option<Box<dyn Fn(I)>> {
        &mut self.0
    }
}

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
