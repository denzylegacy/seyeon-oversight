use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeAction {
    Hold,
    Sell,
    Buy,
}
