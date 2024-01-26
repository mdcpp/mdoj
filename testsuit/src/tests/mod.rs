pub mod empty;

use indicatif::*;
use serde::{Deserialize, Serialize};
use tonic::async_trait;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("assert fail `{0}`")]
    AssertFail(&'static str),
}

#[derive(Default, Serialize, Deserialize)]
pub struct State {
    step: u64,
    pub admin_token: Option<crate::tests::empty::login::AdminToken>,
    #[serde(skip_deserializing, skip_serializing)]
    pub bar: MultiProgress,
}

#[async_trait]
pub trait Test {
    type Error: std::error::Error;
    const NAME: &'static str;
    async fn run(state: &mut State) -> Result<(), Self::Error>;
}

pub async fn run(mut state: State) -> State {
    let title_style = ProgressStyle::with_template("Running {prefix} {wide_msg}").unwrap();

    let title = state.bar.add(ProgressBar::new(1));
    title.set_style(title_style);

    title.set_message(empty::Test::NAME);
    empty::Test::run(&mut state).await.unwrap();

    state.bar.clear().unwrap();
    state
}

// pub struct TaskRunner
