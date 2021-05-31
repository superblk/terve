use std::{fs::File, time::Duration};

use bytes::Bytes;
use reqwest::blocking::Client;

pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub fn new() -> Result<HttpClient, reqwest::Error> {
        let client = Client::builder()
            .user_agent(HTTP_USER_AGENT)
            .connect_timeout(Duration::from_secs(10))
            .https_only(true)
            .build()?;
        Ok(HttpClient { client })
    }

    pub fn download_file(&self, url: &str, mut dest_file: &File) -> Result<u64, reqwest::Error> {
        let num_bytes = self
            .client
            .get(url)
            .header("Accept", "application/octet-stream")
            .send()?
            .error_for_status()?
            .copy_to(&mut dest_file)?;
        Ok(num_bytes)
    }

    pub fn get_bytes(&self, url: &str) -> Result<Bytes, reqwest::Error> {
        let bytes = self
            .client
            .get(url)
            .header("Accept", "application/octet-stream")
            .send()?
            .error_for_status()?
            .bytes()?;
        Ok(bytes)
    }

    pub fn get_text(&self, url: &str, accept: &str) -> Result<String, reqwest::Error> {
        let text = self
            .client
            .get(url)
            .header("Accept", accept)
            .send()?
            .error_for_status()?
            .text()?;
        Ok(text)
    }
}

const HTTP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);
