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

/// 组合的一个持仓版本（报告期）概要。
/// `coverage`：披露口径，"FULL"（中报/年报全量）或 "PARTIAL"（仅前十大+*号行），
/// 由报告期月份 + 行数启发式推断（见 services/position_diff.rs）。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PortfolioVersion {
    pub as_of_date: String,
    pub row_count: i64,
    pub coverage: String,
}

/// 两个版本间的调仓对比结果。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PositionDiff {
    pub from_version: PortfolioVersion,
    pub to_version: PortfolioVersion,
    pub items: Vec<PositionDiffItem>,
}

/// 调仓对比中的一行（一只股票）。
/// `change_type` 是披露名单层面的中立事实（NEW/EXITED/INCREASED/DECREASED/UNCHANGED），
/// "清仓 vs 退出披露"等措辞由前端按两侧披露口径决定。
/// `basis`：分类依据，"shares"（股数主口径）或 "weight"（股数缺失时回退权重）。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PositionDiffItem {
    pub stock_code: String,
    pub stock_name: String,
    pub change_type: String,
    pub basis: String,
    pub from_shares_wan: Option<f64>,
    pub to_shares_wan: Option<f64>,
    pub shares_delta_wan: Option<f64>,
    pub shares_delta_pct: Option<f64>,
    pub from_weight_pct: Option<f64>,
    pub to_weight_pct: Option<f64>,
    pub weight_delta_pp: Option<f64>,
    pub to_market_value_wan: Option<f64>,
    pub from_rank: Option<i64>,
    pub to_rank: Option<i64>,
}
