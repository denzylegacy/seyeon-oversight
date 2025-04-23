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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_ts: Option<u32>,
}

impl Default for HistodayParams {
    fn default() -> Self {
        Self {
            source_sym: String::new(),
            target_sym: String::new(),
            limit: Some(365),
            to_ts: None,
        }
    }
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
    pub rate_limit: Option<RateLimit>,
    #[serde(rename = "Data")]
    pub data: Option<CryptoCompareHistodayData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RateLimit {
    #[serde(rename = "calls_made", default)]
    pub calls_made: Option<CallsInfo>,
    #[serde(rename = "calls_left", default)]
    pub calls_left: Option<CallsInfo>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct CallsInfo {
    #[serde(default)]
    pub second: Option<i32>,
    #[serde(default)]
    pub minute: Option<i32>,
    #[serde(default)]
    pub hour: Option<i32>,
    #[serde(default)]
    pub day: Option<i32>,
    #[serde(default)]
    pub month: Option<i32>,
}

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
