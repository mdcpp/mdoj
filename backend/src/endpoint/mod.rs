use thiserror::Error;

pub mod define;
pub mod problem;

#[derive(Debug, Error)]
pub enum Error {
    #[error("`{0}`")]
    Upstream(#[from] crate::controller::Error),
}
