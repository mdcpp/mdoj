pub mod admin;

use tonic::async_trait;

use super::{ui::UI, Error, State};

pub struct Test;

#[async_trait]
impl super::Test for Test {
    type Error = Error;
    const NAME: &'static str = "simulate user behavior";
    async fn run(state: &mut State) -> Result<(), Self::Error> {
        let mut ui = UI::new(&state.bar, 1);

        ui.inc("submit admin");
        admin::submit(state).await?;

        Ok(())
    }
}
