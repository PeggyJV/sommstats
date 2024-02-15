use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    pub unit_price_in_usomm: u128,
    pub token_decimals: u32,
    pub token_contract: String,
    pub token_symbol: String,
}
