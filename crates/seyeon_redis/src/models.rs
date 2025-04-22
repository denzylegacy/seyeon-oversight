use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeAction {
    Hold,
    Sell,
    Buy
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoStatus {
    pub symbol: String,
    pub action: TradeAction,
    pub sent: bool,
}
