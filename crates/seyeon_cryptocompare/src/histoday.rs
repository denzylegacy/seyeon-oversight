use bon::Builder;
use serde::{Deserialize, Serialize};
use crate::method::Method;

#[derive(Serialize, Deserialize, Debug, Builder)]
#[builder(on(String, into))]
pub struct HistodayParams {
    #[serde(rename = "fsym")]
    pub source_sym: String,

    #[serde(rename = "tsym")]
    pub target_sym: String,
    pub limit: Option<u32>,
    pub to_ts: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CryptoCompareHistodayResponse {
    #[serde(rename = "Response")]
    pub response: String,
    #[serde(rename = "Message")]
    pub message: String,
    #[serde(rename = "HasWarning")]
    pub has_warning: bool,
    #[serde(rename = "Type")]
    pub kind: i64,
    #[serde(rename = "RateLimit")]
    pub rate_limit: RateLimit,
    #[serde(rename = "Data")]
    pub data: CryptoCompareHistodayData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RateLimit {}

#[derive(Serialize, Deserialize, Debug)]
pub struct CryptoCompareHistodayData {
    #[serde(rename = "Aggregated")]
    pub aggregated: bool,
    #[serde(rename = "TimeFrom")]
    pub time_from: i64,
    #[serde(rename = "TimeTo")]
    pub time_to: i64,
    #[serde(rename = "Data")]
    pub data: Vec<CryptoCompareHistodayEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CryptoCompareHistodayEntry {
    pub time: i64,
    pub high: f64,
    pub low: f64,
    pub open: f64,
    pub volumefrom: f64,
    pub volumeto: f64,
    pub close: f64,
    #[serde(rename = "conversionType")]
    pub conversion_type: String,
    #[serde(rename = "conversionSymbol")]
    pub conversion_symbol: String,
}

pub struct Histoday;

impl Method for Histoday {
    const PATH: &'static str = "https://min-api.cryptocompare.com/data/v2/histoday";

    type Response = CryptoCompareHistodayResponse;
    type Params = HistodayParams;
}
