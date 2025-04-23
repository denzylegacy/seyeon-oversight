use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub datetime: DateTime<Utc>,
    pub price: f64,
    pub high: f64,
    pub low: f64,
    pub open: f64,
    pub volume: f64,
}
