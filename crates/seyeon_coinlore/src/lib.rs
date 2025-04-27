pub mod fetch_crypto_data;
pub mod global_market;
pub mod tickers;
pub mod markets;
pub mod exchanges;
pub mod social_stats;

use reqwest::{Client, ClientBuilder};
use serde::de::DeserializeOwned;
use std::collections::HashMap;

// Base URL for Coinlore API
pub const BASE_URL: &str = "https://api.coinlore.net/api";

pub struct CoinloreClient {
    reqwest: Client,
}

impl CoinloreClient {
    pub fn new() -> Self {
        let reqwest = ClientBuilder::new()
            .build()
            .expect("Failed to build reqwest client");

        Self { reqwest }
    }

    // Global market data endpoint
    pub async fn get_global_market_data(&self) -> Result<Vec<global_market::GlobalMarketData>, reqwest::Error> {
        self.get(&format!("{}/global/", BASE_URL)).await
    }

    // Tickers endpoint (all coins with pagination)
    pub async fn get_tickers(&self, start: Option<u32>, limit: Option<u32>) -> Result<tickers::TickersResponse, reqwest::Error> {
        let mut params = HashMap::new();
        
        if let Some(start_val) = start {
            params.insert(String::from("start"), start_val.to_string());
        }
        
        if let Some(limit_val) = limit {
            params.insert(String::from("limit"), limit_val.to_string());
        }
        
        self.get_with_params(&format!("{}/tickers/", BASE_URL), &params).await
    }
    
    // Ticker endpoint (specific coin(s))
    pub async fn get_ticker(&self, ids: &[&str]) -> Result<Vec<tickers::Ticker>, reqwest::Error> {
        let id_param = ids.join(",");
        let mut params = HashMap::new();
        params.insert(String::from("id"), id_param);
        
        self.get_with_params(&format!("{}/ticker/", BASE_URL), &params).await
    }
    
    // Markets for a specific coin
    pub async fn get_coin_markets(&self, coin_id: &str) -> Result<Vec<markets::Market>, reqwest::Error> {
        let mut params = HashMap::new();
        params.insert(String::from("id"), coin_id.to_string());
        
        self.get_with_params(&format!("{}/coin/markets/", BASE_URL), &params).await
    }
    
    // All exchanges
    pub async fn get_exchanges(&self) -> Result<exchanges::ExchangesResponse, reqwest::Error> {
        self.get(&format!("{}/exchanges/", BASE_URL)).await
    }
    
    // Specific exchange by ID
    pub async fn get_exchange(&self, exchange_id: &str) -> Result<exchanges::Exchange, reqwest::Error> {
        let mut params = HashMap::new();
        params.insert(String::from("id"), exchange_id.to_string());
        
        self.get_with_params(&format!("{}/exchange/", BASE_URL), &params).await
    }
    
    // Social stats for a coin
    pub async fn get_social_stats(&self, coin_id: &str) -> Result<social_stats::SocialStats, reqwest::Error> {
        let mut params = HashMap::new();
        params.insert(String::from("id"), coin_id.to_string());
        
        self.get_with_params(&format!("{}/coin/social_stats/", BASE_URL), &params).await
    }

    // Generic GET request
    async fn get<R: DeserializeOwned>(&self, url: &str) -> Result<R, reqwest::Error> {
        let response = self.reqwest.get(url).send().await?;
        let response = response.error_for_status()?;
        
        response.json().await
    }
    
    // GET request with query parameters
    async fn get_with_params<R: DeserializeOwned>(&self, url: &str, params: &HashMap<String, String>) -> Result<R, reqwest::Error> {
        let response = self.reqwest.get(url).query(params).send().await?;
        let response = response.error_for_status()?;
        
        response.json().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_get_global_market_data() {
        let client = CoinloreClient::new();
        let data = client.get_global_market_data().await.expect("Failed to get global market data");
        
        assert!(!data.is_empty());
        println!("Global market data: {:?}", data);
    }
    
    #[tokio::test]
    async fn test_get_tickers() {
        let client = CoinloreClient::new();
        let tickers = client.get_tickers(Some(0), Some(10)).await.expect("Failed to get tickers");
        
        assert!(!tickers.data.is_empty());
        assert!(tickers.data.len() <= 10);
        println!("First ticker: {:?}", tickers.data.first());
    }
    
    #[tokio::test]
    async fn test_get_ticker() {
        let client = CoinloreClient::new();
        let btc = client.get_ticker(&["90"]).await.expect("Failed to get BTC ticker");
        
        assert_eq!(btc.len(), 1);
        assert_eq!(btc[0].symbol, "BTC");
        println!("BTC ticker: {:?}", btc[0]);
    }
}