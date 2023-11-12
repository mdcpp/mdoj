use thiserror::Error;

pub struct SubmitController {}

#[derive(Debug, Error)]
pub enum Error {
    #[error("judger temporarily unavailable")]
    JudgerUnavailable,
    #[error("judger tls error")]
    TlsError,
    #[error("judger health check failed")]
    HealthCheck,
    #[error("judger reach limit")]
    ReachLimit,
    #[error("payload.`{0}` is not a vaild argument")]
    InvaildArgument(&'static str),
}

impl SubmitController {
    pub fn submit(&self) {}
}
