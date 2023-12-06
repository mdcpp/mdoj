use tonic::transport;

pub type Interceptor =
    tonic::service::interceptor::InterceptedService<transport::Channel, TlsIntercept>;

pub struct TlsIntercept {}

pub struct Clients {}
