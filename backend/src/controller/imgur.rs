use std::collections::HashMap;

use reqwest::Client;
use serde::Serialize;

use crate::init::config;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("reqwest error")]
    Reqwest(#[from] reqwest::Error),
    #[error("old API version was used")]
    OldApi,
}

impl From<Error> for tonic::Status {
    fn from(value: Error) -> Self {
        match value {
            Error::Reqwest(x) => tonic::Status::internal(format!("reqwest error: {}", x)),
            Error::OldApi => tonic::Status::internal("old API version was used"),
        }
    }
}

#[derive(Serialize)]
struct AccessTokenRequest<'a> {
    refresh_token: &'a str,
    client_id: &'a str,
    client_secret: &'a str,
    grant_type: &'static str,
}

pub struct ImgurController {
    client: Client,
    config: config::Imgur,
}

impl ImgurController {
    pub fn new(config: &config::Imgur) -> Self {
        let client = Client::new();

        Self {
            client,
            config: config.clone(),
        }
    }
    pub async fn upload(&self, image: Vec<u8>) -> Result<String, Error> {
        let res = self
            .client
            .post("https://api.imgur.com/3/image")
            .body(image)
            .header(
                "Authorization",
                ["Client-ID", &self.config.client_id].concat(),
            )
            .send()
            .await?;
        let payload: HashMap<String, String> = res.json().await?;

        let link = payload.get("link").cloned().ok_or(Error::OldApi)?;

        Ok(link)
    }
}
