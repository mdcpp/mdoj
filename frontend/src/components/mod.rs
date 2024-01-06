pub mod button;
pub mod footer;
pub mod navbar;
pub mod text_input;

pub use button::Button;
pub use footer::Footer;
pub use navbar::Navbar;
pub use text_input::TextInput;

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
