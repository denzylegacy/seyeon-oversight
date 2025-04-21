use crate::method::Method0;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct FearAndGreedIndexResponse {
    #[serde(rename = "lastUpdated")]
    pub last_updated: LastUpdated,
    #[serde(rename = "fgi")]
    pub fgi: Fgi,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LastUpdated {
    #[serde(rename = "epochUnixSeconds")]
    pub epoch_unix_seconds: i64,
    #[serde(rename = "humanDate")]
    pub human_date: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Fgi {
    pub now: FgiValue,
    #[serde(rename = "previousClose")]
    pub previous_close: FgiValue,
    #[serde(rename = "oneWeekAgo")]
    pub one_week_ago: FgiValue,
    #[serde(rename = "oneMonthAgo")]
    pub one_month_ago: FgiValue,
    #[serde(rename = "oneYearAgo")]
    pub one_year_ago: FgiValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FgiValue {
    pub value: i64,
    #[serde(rename = "valueText")]
    pub value_text: String,
}

pub struct FearAndGreedIndex;

impl Method0 for FearAndGreedIndex {
    const PATH: &'static str = "https://fear-and-greed-index.p.rapidapi.com/v1/fgi";
    type Response = FearAndGreedIndexResponse;
}
