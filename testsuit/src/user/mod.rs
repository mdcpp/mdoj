mod login;
use crate::{client::Clients, grpc::*};

struct State {
    token: Option<String>,
    clients: Clients,
}

pub async fn run() {}
