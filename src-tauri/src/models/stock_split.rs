use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StockSplit {
    pub id: i64,
    pub stock_code: String,
    pub split_date: String,
    pub ratio_from: i64,
    pub ratio_to: i64,
    pub created_at: String,
}
