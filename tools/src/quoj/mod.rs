pub mod problem;
pub mod testcases;

use anyhow::Result;
use reqwest::{
    header::{self, HeaderMap},
    Client, IntoUrl, Url,
};
use std::io::{Read, Seek};

#[derive(Debug, Clone)]
pub struct QuojClient {
    base_url: Url,
    client: Client,
}

impl QuojClient {
    pub fn new(base_url: impl IntoUrl, session: String) -> Result<Self> {
        let base_url = base_url.into_url()?;

        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, format!("sessionid={session}").parse()?);

        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build()?;

        Ok(Self { base_url, client })
    }

    pub async fn problem(&self, id: usize) -> Result<problem::ProblemData> {
        problem::problem(&self.client, &self.base_url, id).await
    }

    pub async fn problems(&self) -> Result<Vec<problem::ProblemData>> {
        problem::problems(&self.client, &self.base_url).await
    }

    pub async fn testcases(&self, id: u64) -> Result<testcases::Testcases<impl Read + Seek>> {
        testcases::testcases(&self.client, &self.base_url, id).await
    }
}
