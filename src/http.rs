use std::{error::Error, fs::File, time::Duration};

use reqwest::blocking::Client;

pub fn client() -> Result<Client, Box<dyn Error>> {
    let client = Client::builder()
        .user_agent(HTTP_USER_AGENT)
        .connect_timeout(Duration::from_secs(10))
        .https_only(true)
        .build()?;
    Ok(client)
}

pub fn get_bytes(client: &Client, url: &str, mut dest_file: &File) -> Result<u64, Box<dyn Error>> {
    let num_bytes = client
        .get(url)
        .header("Accept", "application/octet-stream")
        .send()?
        .error_for_status()?
        .copy_to(&mut dest_file)?;
    Ok(num_bytes)
}

pub fn get_text(client: &Client, url: &str, accept: &str) -> Result<String, Box<dyn Error>> {
    let text = client
        .get(url)
        .header("Accept", accept)
        .send()?
        .error_for_status()?
        .text()?;
    Ok(text)
}

const HTTP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);
