use serde::{Deserialize, Serialize};
use tonic::async_trait;

use super::{ui::UI, Error, State};

pub mod problem;

#[derive(Serialize, Deserialize)]
pub struct StartOfId<const L: i32>(pub i32);

pub struct Test;

#[async_trait]
impl super::Test for Test {
    type Error = Error;
    const NAME: &'static str = "add * to *";
    async fn run(state: &mut State) -> Result<(), Self::Error> {
        let mut ui = UI::new(&state.bar, 1);

        ui.inc("create problem");
        problem::testcase(state).await?;

        Ok(())
    }
}
