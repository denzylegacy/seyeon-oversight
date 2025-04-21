pub mod fgi;
mod method;

use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{Client, ClientBuilder};
use serde::de::DeserializeOwned;

pub struct RapidApiClient {
    pub api_key: String,
    reqwest: Client,
}

impl RapidApiClient {
    pub fn new(api_key: &str) -> Self {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(
            "x-rapidapi-key",
            HeaderValue::from_str(api_key).expect("Failed to create header value"),
        );

        default_headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        default_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let reqwest = ClientBuilder::new()
            .default_headers(default_headers)
            .build()
            .expect("Failed to build reqwest client");

        Self {
            api_key: api_key.to_string(),
            reqwest,
        }
    }

    pub(crate) async fn get0<R: DeserializeOwned>(&self, url: &str) -> reqwest::Result<R> {
        let response = self.reqwest.get(url).send().await?;
        let response = response.error_for_status()?;

        response.json().await
    }

    pub async fn call0<M: method::Method0>(&self) -> reqwest::Result<M::Response> {
        self.get0(M::PATH).await
    }
}
