use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type ExchangesResponse = HashMap<String, ExchangeInfo>;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExchangeInfo {
    pub id: String,
    pub name: String,
    pub name_id: String,
    pub volume_usd: f64,
    pub active_pairs: i32,
    pub url: String,
    pub country: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Exchange {
    #[serde(rename = "0")]
    pub info: ExchangeDetail,
    pub pairs: Vec<ExchangePair>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExchangeDetail {
    pub name: String,
    pub date_live: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExchangePair {
    pub base: String,
    pub quote: String,
    pub volume: f64,
    pub price: f64,
    pub price_usd: f64,
    pub time: i64,
}