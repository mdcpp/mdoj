//! test (and fixture) contain empty dataset(database is mainly unchange during the test),
//! except short-lived data(eg. token) and user
//!
//! The purpose of empty test is to ensure basic functionality

use tonic::{async_trait, Code};

pub use super::Error;
use super::{ui::UI, State};

pub mod login;
pub mod problem;

pub struct Test;

#[async_trait]
impl super::Test for Test {
    type Error = Error;
    const NAME: &'static str = "Empty dataset";
    async fn run(state: &mut State) -> Result<(), Self::Error> {
        let mut ui = UI::new(&state.bar, 3);

        ui.inc("list problem(1)");
        problem::list(1, Code::OutOfRange).await?;
        ui.inc("list problem(2)");
        problem::list(1000, Code::InvalidArgument).await?;

        ui.inc("admin login");
        let token = login::login().await?;

        state.admin_token = Some(token);

        Ok(())
    }
}
