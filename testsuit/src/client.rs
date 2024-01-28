use http_body::combinators::UnsyncBoxBody;
use hyper::{client::HttpConnector, header::HeaderValue, Request};

use tonic_web::{GrpcWebCall, GrpcWebClientLayer, GrpcWebClientService};
use tower::{Layer, Service};

pub type Client =
    hyper::Client<HttpConnector, GrpcWebCall<UnsyncBoxBody<hyper::body::Bytes, tonic::Status>>>;

pub struct AuthService<S> {
    token: HeaderValue,
    service: S,
}

impl<S, T> Service<Request<T>> for AuthService<S>
where
    S: Service<Request<T>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<T>) -> Self::Future {
        req.headers_mut().insert("token", self.token.clone());
        self.service.call(req)
    }
}
pub struct AuthLayer {
    token: HeaderValue,
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            token: self.token.clone(),
            service: inner,
        }
    }
}

pub fn connect_with_token(token: String) -> GrpcWebClientService<AuthService<Client>> {
    let client = hyper::Client::builder().build_http();

    let client_w = tower::ServiceBuilder::new()
        .layer(AuthLayer {
            token: token.parse().unwrap(),
        })
        .service(client);

    tower::ServiceBuilder::new()
        .layer(GrpcWebClientLayer::new())
        .service(client_w)
}

pub fn connect() -> GrpcWebClientService<Client> {
    let client = hyper::Client::builder().build_http();

    tower::ServiceBuilder::new()
        .layer(GrpcWebClientLayer::new())
        .service(client)
}
