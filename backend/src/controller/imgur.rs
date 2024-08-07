use std::collections::HashMap;

use reqwest::{multipart, Client};
use tracing::instrument;

use crate::config::CONFIG;
use crate::report_internal;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("reqwest error `{0}`")]
    Reqwest(#[from] reqwest::Error),
    #[error("old API version was used")]
    OldApi,
    #[error("Invaild image")]
    InvaildImage,
}

impl From<Error> for tonic::Status {
    fn from(value: Error) -> Self {
        match value {
            Error::InvaildImage => tonic::Status::failed_precondition("Invaild image"),
            _ => report_internal!(error, value),
        }
    }
}

// FIXME: use token to delete image
// /// json serialization for imgur api
// ///
// /// Read Imgur API Docs for more
// #[derive(Serialize)]
// struct AccessTokenRequest<'a> {
//     refresh_token: &'a str,
//     client_id: &'a str,
//     client_secret: &'a str,
//     grant_type: &'static str,
// }

pub struct ImgurController {
    client: Client,
    client_id: String,
}

impl Default for ImgurController {
    fn default() -> Self {
        Self::new()
    }
}

impl ImgurController {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            client_id: CONFIG.imgur.client_id.to_string(),
        }
    }
    /// upload image
    #[instrument(skip_all, level = "debug")]
    pub async fn upload(&self, image: Vec<u8>) -> Result<String, Error> {
        // check max image size(10MB)
        if image.len() >= 10 * 1000 * 1000 {
            return Err(Error::InvaildImage);
        }

        let part = multipart::Part::bytes(image).file_name("upload.bin");
        let form = multipart::Form::new().part("image", part);

        let res = self
            .client
            .post("https://api.imgur.com/3/image")
            .multipart(form)
            .header("Authorization", ["Client-ID", &self.client_id].concat())
            .send()
            .await?;

        let payload: HashMap<String, String> = res.json().await?;

        let link = payload.get("link").cloned().ok_or(Error::OldApi)?;

        Ok(link)
    }
}
