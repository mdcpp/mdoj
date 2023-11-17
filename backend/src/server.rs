use std::sync::Arc;

use crate::controller::{duplicate::DupController, *};

pub struct Server {
    pub token: Arc<token::TokenController>,
    pub submit: Arc<submit::SubmitController>,
    pub dup: DupController,
}
