use std::sync::Arc;

use crate::controller::*;

pub struct Server {
    pub controller: Arc<token::TokenController>,
    pub submit: Arc<submit::SubmitController>,
}
