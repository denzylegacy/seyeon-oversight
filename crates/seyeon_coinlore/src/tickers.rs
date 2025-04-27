use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct TickersResponse {
    pub data: Vec<Ticker>,
    pub info: Info,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Info {
    pub coins_num: i32,
    pub time: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Ticker {
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub nameid: String,
    pub rank: i32,
    pub price_usd: String,
    pub percent_change_24h: String,
    pub percent_change_1h: String,
    pub percent_change_7d: String,
    pub price_btc: String,
    pub market_cap_usd: String,
    #[serde(rename = "volume24")]
    pub volume24: f64,
    #[serde(rename = "volume24a")]
    pub volume24a: Option<f64>,
    pub csupply: String,
    pub tsupply: Option<String>,
    pub msupply: Option<String>,
}