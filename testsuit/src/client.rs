// use std::sync::Arc;

// use crate::grpc::backend::token_set_client::TokenSetClient;
// use hyper::{client::HttpConnector, Uri};
// use rustls::client::{DangerousClientConfig, ServerCertVerifier, ServerCertVerified};
// use rustls_pki_types::*;
// use tokio_rustls::rustls::*;

// #[derive(Debug)]
// pub struct AllowAllVerifier;

// impl ServerCertVerifier for AllowAllVerifier {
//     fn supported_verify_schemes(&self) -> Vec<tokio_rustls::rustls::SignatureScheme> {
//         vec![tokio_rustls::rustls::SignatureScheme::RSA_PSS_SHA256]
//     }

//     fn verify_server_cert(
//         &self,
//         end_entity: &Certificate,
//         intermediates: &[Certificate],
//         server_name: &rustls::ServerName,
//         scts: &mut dyn Iterator<Item = &[u8]>,
//         ocsp_response: &[u8],
//         now: std::time::SystemTime,
//     ) -> Result<ServerCertVerified, Error> {
//         todo!()
//     }
// }

// fn tls_config() -> ClientConfig {
//     let verifier = Arc::new(AllowAllVerifier) as Arc<(dyn ServerCertVerifier + 'static)>;
//     let mut tls_config = ClientConfig::builder().with_safe_defaults().with_no_client_auth()
// ;
//         // .with_custom_certificate_verifier(verifier)
//         // .with_no_client_auth();
//     tls_config
// }

// pub struct Clients {}

// async fn token_client() {
//     // let tls = tls_config();

//     // let mut http = HttpConnector::new();
//     // http.enforce_http(false);

//     // // We have to do some wrapping here to map the request type from
//     // // `https://example.com` -> `https://[::1]:50051` because `rustls`
//     // // doesn't accept ip's as `ServerName`.
//     // let connector = tower::ServiceBuilder::new()
//     //     .layer_fn(move |s| {
//     //         let tls = tls.clone();

//     //         hyper_rustls::HttpsConnectorBuilder::new()
//     //             .with_tls_config(tls)
//     //             .https_or_http()
//     //     })
//     //     // Since our cert is signed with `example.com` but we actually want to connect
//     //     // to a local server we will override the Uri passed from the `HttpsConnector`
//     //     // and map it to the correct `Uri` that will connect us directly to the local server.
//     //     .map_request(|_| Uri::from_static("https://[::1]:50051"))
//     //     .service(http);
// }
