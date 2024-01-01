use http_body::combinators::UnsyncBoxBody;
use hyper::client::HttpConnector;
use tonic_web::{GrpcWebCall, GrpcWebClientLayer, GrpcWebClientService};

pub type Intercepter = GrpcWebClientService<
    hyper::Client<HttpConnector, GrpcWebCall<UnsyncBoxBody<hyper::body::Bytes, tonic::Status>>>,
>;

pub fn connection() -> Intercepter {
    let client = hyper::Client::builder().build_http();

    tower::ServiceBuilder::new()
        .layer(GrpcWebClientLayer::new())
        .service(client)
}
