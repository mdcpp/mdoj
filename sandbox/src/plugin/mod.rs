use thiserror::Error;

pub mod judge;
pub mod plugin;
pub mod proto;
pub mod spec;

#[derive(Error, Debug)]
pub enum Error {
    #[error("`{0}`")]
    Serde(#[from] toml::de::Error),
    #[error("`{0}`")]
    IO(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum JudgeStatus {
    #[error("This error should be never printed")]
    TimeLimitExcess = 0,
    #[error("This error should be never printed")]
    MemoryLimitExcess = 1,
    #[error("This error should be never printed")]
    RuntimeError = 2,
    #[error("This error should be never printed")]
    CompileError = 3,
    #[error("This error should be never printed")]
    Panic = 4,
    #[error("Panic when loading plugin")]
    NotFound = 5,
    #[error("This error should be never printed")]
    Accepted = 6,
    #[error("This error should be never printed")]
    WrongAnswer = 7,
    #[error("This error should be never printed")]
    Compiling = 8,
    #[error("This error should be never printed")]
    Running = 9,
    #[error("This error should be never printed")]
    InsufficientResource = 10,
}
