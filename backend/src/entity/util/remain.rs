use crate::util::error::Error;

pub trait Remain {
    fn remain(&self, max: usize) -> Result<usize, Error>;
}
