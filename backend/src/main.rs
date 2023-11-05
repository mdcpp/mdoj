use controller::token::TokenController;

pub mod controller;
pub mod endpoint;
pub mod grpc;
pub mod init;

#[derive(Default)]
pub struct Server {
    pub controller: TokenController,
}

fn main() {}
