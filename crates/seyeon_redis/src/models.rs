use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TradeAction {
    Hold,
    Sell,
    Buy,
    Any
}

impl fmt::Display for TradeAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TradeAction::Hold => write!(f, "Hold"),
            TradeAction::Sell => write!(f, "Sell"),
            TradeAction::Buy => write!(f, "Buy"),
            TradeAction::Any => write!(f, "Any"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoStatus {
    pub symbol: String,
    pub action: TradeAction,
    pub sent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportStatus {
    pub last_report_date: String,
    pub report_sent_today: bool,
}

impl Default for ReportStatus {
    fn default() -> Self {
        Self {
            last_report_date: "2000-01-01".to_string(),
            report_sent_today: false,
        }
    }
}
