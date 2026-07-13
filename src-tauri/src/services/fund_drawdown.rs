//! 基金最大回撤 + 定投择时信号计算（纯函数，读时现算，不落库）。
//!
//! 回撤基准为复权净值（`FundNavPoint.adjusted_nav`）。回撤算法复用
//! `performance_service::calculate_max_drawdown`（把复权净值当作 `portfolio_value`
//! 序列），避免重复实现峰值/谷值/修复日期逻辑。
//!
//! 信号（无状态即时派生，回撤/当前回撤均为**负百分比**）：
//! - `BUY_ZONE`：当前回撤 CDD ≤ 历史最大回撤 HMDD（净值跌到历史大底）→ 建议开启定投
//! - `APPROACHING`：CDD ≤ HMDD × 0.9（接近历史大底）→ 预警
//! - `NORMAL`：其余；净值由定投区回升越过信号线后即回到此档，对应"停止定投"

use crate::models::performance::ReturnDataPoint;
use crate::models::{DrawdownWindow, FundDrawdownAnalysis, FundNavPoint};
use crate::services::performance_service::calculate_max_drawdown;

/// 接近区系数：当前回撤达到历史最大回撤的该比例即预警。
const APPROACHING_RATIO: f64 = 0.9;

/// 把净值点转成 performance 的 `ReturnDataPoint`（只有 `date` 与 `portfolio_value` 参与回撤）。
fn to_points(nav: &[FundNavPoint]) -> Vec<ReturnDataPoint> {
    nav.iter()
        .map(|p| ReturnDataPoint {
            date: p.nav_date.clone(),
            cumulative_return: 0.0,
            daily_return: 0.0,
            portfolio_value: p.adjusted_nav,
            daily_pnl: 0.0,
        })
        .collect()
}

/// latest(`YYYY-MM-DD`) 回溯 `years` 年的 cutoff 字符串（年份相减、月日保留；
/// 闰日边界用字符串比较可忽略）。解析失败返回极小日期（等价全历史）。
fn cutoff_date(latest: &str, years: i64) -> String {
    if latest.len() >= 4 {
        if let Ok(y) = latest[..4].parse::<i64>() {
            return format!("{:04}{}", y - years, &latest[4..]);
        }
    }
    "0000-00-00".to_string()
}

/// 取最近 `years` 自然年内的净值点（nav 升序，按最新日期回溯）。
fn window_slice(nav: &[FundNavPoint], years: i64) -> &[FundNavPoint] {
    let latest = match nav.last() {
        Some(p) => &p.nav_date,
        None => return nav,
    };
    let cutoff = cutoff_date(latest, years);
    let start = nav.partition_point(|p| p.nav_date < cutoff);
    &nav[start..]
}

/// 两个 `YYYY-MM-DD` 之间的自然日跨度。
fn span_days(start: &str, end: &str) -> Option<i64> {
    use chrono::NaiveDate;
    let s = NaiveDate::parse_from_str(start, "%Y-%m-%d").ok()?;
    let e = NaiveDate::parse_from_str(end, "%Y-%m-%d").ok()?;
    Some((e - s).num_days())
}

/// 按基金类型给出适用性提示；主动权益（混合/股票型）为主要适用对象，返回 None。
fn applicability_note(fund_type: Option<&str>) -> Option<String> {
    let t = fund_type?;
    if t.contains("货币") {
        Some("货币基金几乎无回撤，最大回撤定投信号不适用".to_string())
    } else if t.contains("债") {
        Some("债券基金回撤较小，信号参考意义有限".to_string())
    } else if t.contains("指数") || t.contains("ETF") || t.contains("被动") {
        Some("被动指数基金：策略同样适用，但建议结合指数估值判断".to_string())
    } else {
        None
    }
}

/// 计算某基金的最大回撤与定投信号。`nav` 必须按 `nav_date` 升序。
pub fn analyze(
    fund_code: &str,
    fund_type: Option<String>,
    nav: &[FundNavPoint],
) -> Result<FundDrawdownAnalysis, String> {
    if nav.is_empty() {
        return Err("该基金尚无净值数据，请先刷新净值".to_string());
    }

    let full = calculate_max_drawdown(&to_points(nav));

    let start_date = nav.first().unwrap().nav_date.clone();
    let latest = nav.last().unwrap();
    let latest_date = latest.nav_date.clone();
    let latest_adjusted_nav = latest.adjusted_nav;

    // 历史最高复权净值（当前运行峰值）。
    let peak_nav = nav.iter().map(|p| p.adjusted_nav).fold(f64::MIN, f64::max);

    // 当前回撤（负百分比）。
    let current_drawdown = if peak_nav > 0.0 {
        (latest_adjusted_nav - peak_nav) / peak_nav * 100.0
    } else {
        0.0
    };

    let hmdd = full.max_drawdown; // 负百分比
    // 信号线（复权净值）L = 峰值 × (1 − |HMDD|)；hmdd 为负，故 1 + hmdd/100 = 1 − |hmdd|/100。
    let threshold_nav = peak_nav * (1.0 + hmdd / 100.0);
    let distance_to_signal_pct = if latest_adjusted_nav > 0.0 {
        (latest_adjusted_nav - threshold_nav) / latest_adjusted_nav * 100.0
    } else {
        0.0
    };

    // 信号线对应的单位净值：当下同一时点单位/复权净值比例固定，按该比例把信号线折回单位净值口径，
    // 供用户对照平台申购净值（假设期间无分红）。
    let latest_unit_nav = latest.unit_nav;
    let threshold_unit_nav = if latest_adjusted_nav > 0.0 {
        latest_unit_nav.map(|u| u * threshold_nav / latest_adjusted_nav)
    } else {
        None
    };

    // 三档信号（负值比较；更深的回撤=更小的负数）。hmdd≈0（如货基）直接 NORMAL。
    let signal_state = if hmdd >= -1e-9 {
        "NORMAL"
    } else if current_drawdown <= hmdd {
        "BUY_ZONE"
    } else if current_drawdown <= hmdd * APPROACHING_RATIO {
        "APPROACHING"
    } else {
        "NORMAL"
    }
    .to_string();

    // 对照窗口：全历史 + 近 5 年 + 近 3 年。
    let mut windows = vec![DrawdownWindow {
        label: "全历史".to_string(),
        max_drawdown: hmdd,
        peak_date: full.peak_date.clone(),
        trough_date: full.trough_date.clone(),
        recovery_date: full.recovery_date.clone(),
    }];
    for (label, years) in [("近5年", 5i64), ("近3年", 3i64)] {
        let slice = window_slice(nav, years);
        if slice.len() >= 2 {
            let w = calculate_max_drawdown(&to_points(slice));
            windows.push(DrawdownWindow {
                label: label.to_string(),
                max_drawdown: w.max_drawdown,
                peak_date: w.peak_date,
                trough_date: w.trough_date,
                recovery_date: w.recovery_date,
            });
        }
    }

    let history_too_short = span_days(&start_date, &latest_date).is_some_and(|d| d < 365);

    Ok(FundDrawdownAnalysis {
        fund_code: fund_code.to_string(),
        fund_type: fund_type.clone(),
        start_date,
        latest_date,
        latest_adjusted_nav,
        latest_unit_nav,
        peak_nav,
        max_drawdown: hmdd,
        peak_date: full.peak_date,
        trough_date: full.trough_date,
        recovery_date: full.recovery_date,
        current_drawdown,
        threshold_nav,
        threshold_unit_nav,
        distance_to_signal_pct,
        signal_state,
        approaching_ratio: APPROACHING_RATIO,
        windows,
        history_too_short,
        applicability_note: applicability_note(fund_type.as_deref()),
        drawdown_series: full.drawdown_series,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nav(points: &[(&str, f64)]) -> Vec<FundNavPoint> {
        points
            .iter()
            .map(|(d, v)| FundNavPoint {
                nav_date: d.to_string(),
                unit_nav: Some(*v),
                acc_nav: Some(*v),
                adjusted_nav: *v,
                daily_return: None,
            })
            .collect()
    }

    /// 复权净值与单位净值分叉的构造器（模拟有分红的基金）。
    fn mk(date: &str, adjusted: f64, unit: f64) -> FundNavPoint {
        FundNavPoint {
            nav_date: date.to_string(),
            unit_nav: Some(unit),
            acc_nav: Some(unit),
            adjusted_nav: adjusted,
            daily_return: None,
        }
    }

    #[test]
    fn test_threshold_unit_nav_conversion() {
        // 复权序列 100→50→100→80(最新)，HMDD=-50%、信号线复权=50；
        // 最新单位净值 4.0（复权 80），信号线对应单位净值 = 4.0 × 50/80 = 2.5。
        let s = vec![
            mk("2020-01-01", 100.0, 5.0),
            mk("2021-01-01", 50.0, 2.5),
            mk("2022-01-01", 100.0, 5.0),
            mk("2023-01-01", 80.0, 4.0),
        ];
        let a = analyze("161005", None, &s).unwrap();
        assert!((a.threshold_nav - 50.0).abs() < 1e-6);
        assert_eq!(a.latest_unit_nav, Some(4.0));
        assert!((a.threshold_unit_nav.unwrap() - 2.5).abs() < 1e-6);
    }

    #[test]
    fn test_empty_is_error() {
        assert!(analyze("000000", None, &[]).is_err());
    }

    #[test]
    fn test_normal_after_recovery() {
        // 100→50(−50%)→100→120(峰)→115(最新)：当前回撤仅 −4.17%。
        let s = nav(&[
            ("2020-01-01", 100.0),
            ("2021-01-01", 50.0),
            ("2022-01-01", 100.0),
            ("2023-01-01", 120.0),
            ("2024-01-01", 115.0),
        ]);
        let a = analyze("100056", Some("混合型-偏股".to_string()), &s).unwrap();
        assert!((a.max_drawdown - (-50.0)).abs() < 1e-6);
        assert!((a.peak_nav - 120.0).abs() < 1e-6);
        assert!((a.current_drawdown - (-4.166666)).abs() < 1e-3);
        assert!((a.threshold_nav - 60.0).abs() < 1e-6); // 120 × (1−0.5)
        assert!(a.distance_to_signal_pct > 0.0); // 尚需下跌才触线
        assert_eq!(a.signal_state, "NORMAL");
        assert!(!a.history_too_short);
        assert!(a.applicability_note.is_none()); // 主动权益 → 适用
        // 全历史 + 近5年 + 近3年
        assert!(a.windows.iter().any(|w| w.label == "全历史"));
    }

    #[test]
    fn test_buy_zone_at_historical_bottom() {
        // 100→50→100→50(最新)：当前回撤 −50% = 历史最大回撤 → 触线。
        let s = nav(&[
            ("2020-01-01", 100.0),
            ("2021-01-01", 50.0),
            ("2022-01-01", 100.0),
            ("2023-01-01", 50.0),
        ]);
        let a = analyze("161005", None, &s).unwrap();
        assert!((a.max_drawdown - (-50.0)).abs() < 1e-6);
        assert!((a.current_drawdown - (-50.0)).abs() < 1e-6);
        assert_eq!(a.signal_state, "BUY_ZONE");
        assert!(a.distance_to_signal_pct <= 1e-6); // 已在触线（≤0）
    }

    #[test]
    fn test_approaching_zone() {
        // 100→50→100→54(最新)：当前回撤 −46%，介于 −45%(=−50×0.9) 与 −50% 之间。
        let s = nav(&[
            ("2020-01-01", 100.0),
            ("2021-01-01", 50.0),
            ("2022-01-01", 100.0),
            ("2023-01-01", 54.0),
        ]);
        let a = analyze("163417", None, &s).unwrap();
        assert!((a.current_drawdown - (-46.0)).abs() < 1e-6);
        assert_eq!(a.signal_state, "APPROACHING");
    }

    #[test]
    fn test_zero_drawdown_money_fund_is_normal() {
        // 单调上涨（货基式）：HMDD≈0 → NORMAL，且带适用性提示。
        let s = nav(&[
            ("2020-01-01", 100.0),
            ("2021-01-01", 101.0),
            ("2022-01-01", 102.0),
        ]);
        let a = analyze("000001", Some("货币型".to_string()), &s).unwrap();
        assert!(a.max_drawdown.abs() < 1e-6);
        assert_eq!(a.signal_state, "NORMAL");
        assert!(a.applicability_note.unwrap().contains("货币"));
    }

    #[test]
    fn test_history_too_short_flag() {
        let s = nav(&[("2026-01-05", 1.0), ("2026-06-05", 0.8)]);
        let a = analyze("123456", None, &s).unwrap();
        assert!(a.history_too_short); // 跨度 < 365 天
    }

    #[test]
    fn test_window_slice_recent_years() {
        // 10 年序列，近3年窗口只含最后 4 点。
        let s = nav(&[
            ("2016-01-01", 100.0),
            ("2022-01-01", 100.0),
            ("2023-06-01", 90.0),
            ("2024-06-01", 80.0),
            ("2025-06-01", 95.0),
        ]);
        let w3 = window_slice(&s, 3); // cutoff = 2022-06-01
        assert_eq!(w3.len(), 3); // 2023-06-01, 2024-06-01, 2025-06-01
        assert_eq!(w3[0].nav_date, "2023-06-01");
    }
}
