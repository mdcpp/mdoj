use thiserror::Error;

pub mod problem;
pub mod util;

#[derive(Debug, Error)]
pub enum Error {
    #[error("`{0}`")]
    Upstream(#[from] crate::controller::Error),
    #[error("Premission Deny")]
    PremissionDeny,
}
