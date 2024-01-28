//! test (and fixture) to create dataset.
//!
//! Include verification of role logic
//!
//! Does not include verification of data existance nor data accessibility

use serde::{Deserialize, Serialize};
use tonic::async_trait;

use super::{ui::UI, Error, State};

pub mod contest;
pub mod problem;
pub mod testcase;
pub mod user;

#[derive(Serialize, Deserialize)]
pub struct StartOfId<const L: i32>(pub i32);

pub struct Test;

#[async_trait]
impl super::Test for Test {
    type Error = Error;
    const NAME: &'static str = "sample dataset";
    async fn run(state: &mut State) -> Result<(), Self::Error> {
        let mut ui = UI::new(&state.bar, 4);

        ui.inc("create problem");
        problem::create(state).await?;
        ui.inc("create testcase");
        testcase::create(state).await?;
        ui.inc("create contest");
        contest::create(state).await?;
        ui.inc("create user");
        user::create(state).await?;

        Ok(())
    }
}
