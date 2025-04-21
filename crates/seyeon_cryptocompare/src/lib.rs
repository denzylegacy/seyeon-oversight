pub mod histoday;
pub mod method;

use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, ClientBuilder};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct CryptocompareClient {
    reqwest: Client,
}

impl CryptocompareClient {
    pub fn new(api_key: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(format!("Apikey {api_key}").as_str())
                .expect("Failed to create header value"),
        );

        let reqwest = ClientBuilder::new()
            .default_headers(headers)
            .build()
            .expect("Failed to build reqwest client");

        Self { reqwest }
    }

    pub(crate) async fn get<T: DeserializeOwned, P: Serialize + ?Sized>(
        &self,
        url: &str,
        params: &P,
    ) -> reqwest::Result<T> {
        let response = self
            .reqwest
            .get(url)
            .query(params)
            .send()
            .await?
            .error_for_status()?
            .json::<T>()
            .await?;

        Ok(response)
    }

    pub async fn call<M: method::Method>(&self, params: M::Params) -> reqwest::Result<M::Response> {
        self.get(M::PATH, &params).await
    }
}
