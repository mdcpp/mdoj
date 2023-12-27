use std::{collections::HashSet, net::IpAddr, time::Duration};

use super::{ConnectionDetail, Error};
use crate::init::config;
use hickory_resolver::TokioAsyncResolver;

use super::{Routable, RouteStatus};

/// Upstream source for docker swarm
pub struct SwarmRouter {
    dns: String,
    secret: Option<String>,
    address: HashSet<IpAddr>,
    resolver: TokioAsyncResolver,
}

fn to_uri(ip: &IpAddr) -> String {
    match ip {
        IpAddr::V4(ip) => format!("http://{}", ip),
        IpAddr::V6(ip) => format!("http://[{}]", ip),
    }
}

#[tonic::async_trait]
impl Routable for SwarmRouter {
    async fn route(&mut self) -> Result<RouteStatus, Error> {
        let result = self.resolver.lookup_ip(self.dns.as_str()).await?;
        let ips = result.as_lookup().records().iter().filter_map(|x| {
            let data = x.data()?;
            data.ip_addr()
        });

        for ip in ips {
            if !self.address.contains(&ip) {
                let uri = to_uri(&ip);
                self.address.insert(ip);
                return Ok(RouteStatus::NewConnection(ConnectionDetail {
                    uri,
                    secret: self.secret.clone(),
                    reuse: true,
                }));
            }
        }
        return Ok(RouteStatus::Wait(Duration::from_secs(30)));
    }

    fn new(config: config::Judger) -> Result<Self, Error> {
        Ok(Self {
            dns: config.name,
            secret: config.secret,
            address: HashSet::new(),
            resolver: TokioAsyncResolver::tokio_from_system_conf()?,
        })
    }
}
