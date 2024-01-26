//! test (and fixture) contain empty dataset(database is mainly unchange during the test),
//! except short-lived data(eg. token) and user
//!
//! The purpose of empty test is to ensure basic functionality

use indicatif::{ProgressBar, ProgressStyle};
use tonic::{async_trait, Code};

pub use super::Error;
use super::State;

pub mod login;
pub mod problem;

pub struct Test;

#[async_trait]
impl super::Test for Test {
    type Error = Error;
    const NAME: &'static str = "Empty dataset";
    async fn run(state: &mut State) -> Result<(), Self::Error> {
        let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");

        let pb = state.bar.add(ProgressBar::new(4));
        pb.set_style(spinner_style.clone());
        pb.set_prefix(format!("[{}/?]", 4));

        problem::list(1, Code::NotFound).await?;
        problem::list(1000, Code::InvalidArgument).await?;

        let token = login::login().await?;

        state.admin_token = Some(token);

        Ok(())
    }
}
