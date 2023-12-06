use crate::case::Case;

use super::State;

pub struct AdminLogin;

#[tonic::async_trait]
impl Case<State> for AdminLogin {
    const NAME: &'static str = "login as admin@admin";

    async fn run(&self, state: &mut State) -> Result<(), String> {
        todo!()
    }
}
