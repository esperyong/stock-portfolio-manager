use crate::models::performance::DrawdownPoint;
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

/// 基金净值历史中的一天（`fund_nav_history` 一行）。
/// `adjusted_nav`（复权净值）为回撤计算基准，由日复权收益率累乘重建；
/// `unit_nav`/`acc_nav` 原样保存备查。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FundNavPoint {
    pub nav_date: String,
    pub unit_nav: Option<f64>,
    pub acc_nav: Option<f64>,
    pub adjusted_nav: f64,
    pub daily_return: Option<f64>,
}

/// 某一窗口口径下的最大回撤概要（全历史 / 近 5 年 / 近 3 年）。
/// `max_drawdown` 为负百分比（如 -52.2 表示 -52.2%）。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DrawdownWindow {
    pub label: String,
    pub max_drawdown: f64,
    pub peak_date: String,
    pub trough_date: String,
    pub recovery_date: Option<String>,
}

/// 基金最大回撤 + 定投择时信号分析（读时现算，不落库）。
/// 回撤/当前回撤均以复权净值计算，值为负百分比。
/// `signal_state`：`NORMAL` / `APPROACHING`（接近历史大底）/ `BUY_ZONE`（建议开启定投）。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FundDrawdownAnalysis {
    pub fund_code: String,
    pub fund_type: Option<String>,
    pub start_date: String,
    pub latest_date: String,
    /// 最新复权净值。
    pub latest_adjusted_nav: f64,
    /// 历史最高复权净值（当前运行峰值）。
    pub peak_nav: f64,
    /// 全历史最大回撤（负百分比，HMDD）。
    pub max_drawdown: f64,
    pub peak_date: String,
    pub trough_date: String,
    pub recovery_date: Option<String>,
    /// 当前回撤（负百分比，CDD = (最新−峰值)/峰值）。
    pub current_drawdown: f64,
    /// 历史最大回撤信号线净值 L = 峰值 × (1 − |HMDD|)。
    pub threshold_nav: f64,
    /// 距触线还需下跌的百分比（正=尚需下跌该幅度；≤0=已在触线下方）。
    pub distance_to_signal_pct: f64,
    pub signal_state: String,
    /// 接近区系数（默认 0.9）。
    pub approaching_ratio: f64,
    /// 对照窗口（全历史 / 近 5 年 / 近 3 年）。
    pub windows: Vec<DrawdownWindow>,
    /// 净值历史不足一年时为 true，信号参考意义有限。
    pub history_too_short: bool,
    /// 按基金类型给出的适用性提示（主动权益基金为 None）。
    pub applicability_note: Option<String>,
    /// 全历史逐日回撤序列（供水下曲线展示）。
    pub drawdown_series: Vec<DrawdownPoint>,
}
