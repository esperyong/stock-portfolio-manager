use serde::{Deserialize, Serialize};

/// 组合：权重配比的版本序列（无成本概念）。
/// `source_type='FUND'` 时组合来自公募基金定期披露，`fund_code` 非空；
/// `latest_as_of_date` 为查询时计算的最新版本日期（非表字段）。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Portfolio {
    pub id: String,
    pub name: String,
    pub source_type: String,
    pub fund_code: Option<String>,
    pub fund_type: Option<String>,
    pub latest_as_of_date: Option<String>,
    pub last_refreshed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 仓位：组合在某一版本（`as_of_date`）下的单行持仓。
/// 权重为一等公民；股数（万股）与市值（万元）为基金来源的附加事实。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PortfolioPosition {
    pub id: i64,
    pub portfolio_id: String,
    pub as_of_date: String,
    pub stock_code: String,
    pub stock_name: String,
    pub weight_pct: Option<f64>,
    pub shares_wan: Option<f64>,
    pub market_value_wan: Option<f64>,
    pub position_rank: Option<i64>,
    pub created_at: String,
}

/// 天天基金 fundsuggest 搜索接口的单条候选。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FundSearchResult {
    pub fund_code: String,
    pub fund_name: String,
    pub fund_type: String,
}
