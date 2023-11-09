use thiserror::Error;

pub struct SubmitController {}

#[derive(Debug, Error)]
pub enum Error {}

impl SubmitController {
    pub fn submit(&self) {}
}
