use serde::{Deserialize, Serialize};
use reqwest;

#[derive(Serialize, Deserialize, Debug)]
pub struct GlobalMarketData {
    pub coins_count: i64,
    pub active_markets: i64,
    pub total_mcap: f64,
    pub total_volume: f64,
    pub btc_d: String,
    pub eth_d: String,
    pub mcap_change: String,
    pub volume_change: String,
    pub avg_change_percent: String,
    pub volume_ath: f64,
    pub mcap_ath: f64,
}

pub const GLOBAL_MARKET_ENDPOINT: &str = "https://api.coinlore.net/api/global/";

/// Fetch global cryptocurrency market data from the Coinlore API
pub async fn get_global_data() -> Result<GlobalMarketData, reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client.get(GLOBAL_MARKET_ENDPOINT).send().await?;
    
    // The API returns an array with a single object
    let mut data: Vec<GlobalMarketData> = response.json().await?;
    
    // Return the first (and only) item in the array
    Ok(data.remove(0))
}