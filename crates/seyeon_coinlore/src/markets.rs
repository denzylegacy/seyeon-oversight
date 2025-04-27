use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Market {
    pub name: String,
    pub base: String,
    pub quote: String,
    pub price: f64,
    pub price_usd: f64,
    pub volume: f64,
    pub volume_usd: f64,
    pub time: i64,
}