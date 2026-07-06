//! 调仓 diff：两个持仓版本间的逐股变动计算。
//!
//! 全部为无 IO 纯函数，命令层负责取数。核心口径约定（见 design D1~D3）：
//! - 分类以持股数（万股）为主口径，权重口径会被股价涨跌污染，仅作辅助展示；
//! - `change_type` 只描述披露名单层面的中立事实，"清仓 vs 退出披露"等措辞
//!   由展示层按两侧披露口径决定；
//! - 两侧共同出现的股票，其股数对比不受披露口径影响（部分披露中的股数是真实值）。

use crate::models::{PortfolioPosition, PositionDiffItem};
use std::collections::HashMap;

/// 6/12 月版本判定为全量披露的最小行数。
/// 实测：部分披露（前十大+*号行）最多约 26 行，全量最少约 60 行，中间地带宽。
pub const FULL_COVERAGE_MIN_ROWS: i64 = 35;

/// 股数持平判定阈值（万股）。上游数据精确到 0.01 万股。
const SHARES_EPSILON: f64 = 0.001;

/// 权重口径（回退时）的持平判定阈值（百分点）。
const WEIGHT_EPSILON: f64 = 0.001;

/// 推断某版本的披露口径。
/// 3/9 月只存在季报（恒为前十大口径）；6/12 月在中报/年报发布后才变为全量，
/// 以行数阈值区分。误判方向保守（全量被判部分只影响措辞，不虚构"清仓"）。
pub fn infer_coverage(as_of_date: &str, row_count: i64) -> String {
    let month = as_of_date.get(5..7).unwrap_or("");
    match month {
        "03" | "09" => "PARTIAL".to_string(),
        _ => {
            if row_count >= FULL_COVERAGE_MIN_ROWS {
                "FULL".to_string()
            } else {
                "PARTIAL".to_string()
            }
        }
    }
}

/// 计算两个版本间的逐股变动。
/// 返回顺序：NEW → INCREASED → DECREASED → EXITED → UNCHANGED，
/// 类别内按 |权重变化| 降序（缺失权重的排在类别末尾）。
pub fn compute_diff(
    from_rows: &[PortfolioPosition],
    to_rows: &[PortfolioPosition],
) -> Vec<PositionDiffItem> {
    let from_map: HashMap<&str, &PortfolioPosition> =
        from_rows.iter().map(|r| (r.stock_code.as_str(), r)).collect();
    let to_map: HashMap<&str, &PortfolioPosition> =
        to_rows.iter().map(|r| (r.stock_code.as_str(), r)).collect();

    let mut items: Vec<PositionDiffItem> = Vec::new();

    for to_row in to_rows {
        match from_map.get(to_row.stock_code.as_str()) {
            Some(from_row) => items.push(diff_matched(from_row, to_row)),
            None => items.push(PositionDiffItem {
                stock_code: to_row.stock_code.clone(),
                stock_name: to_row.stock_name.clone(),
                change_type: "NEW".to_string(),
                basis: "shares".to_string(),
                from_shares_wan: None,
                to_shares_wan: to_row.shares_wan,
                shares_delta_wan: to_row.shares_wan,
                shares_delta_pct: None,
                from_weight_pct: None,
                to_weight_pct: to_row.weight_pct,
                weight_delta_pp: to_row.weight_pct,
                to_market_value_wan: to_row.market_value_wan,
                from_rank: None,
                to_rank: to_row.position_rank,
            }),
        }
    }

    for from_row in from_rows {
        if !to_map.contains_key(from_row.stock_code.as_str()) {
            items.push(PositionDiffItem {
                stock_code: from_row.stock_code.clone(),
                stock_name: from_row.stock_name.clone(),
                change_type: "EXITED".to_string(),
                basis: "shares".to_string(),
                from_shares_wan: from_row.shares_wan,
                to_shares_wan: None,
                shares_delta_wan: from_row.shares_wan.map(|s| -s),
                shares_delta_pct: from_row.shares_wan.map(|_| -100.0),
                from_weight_pct: from_row.weight_pct,
                to_weight_pct: None,
                weight_delta_pp: from_row.weight_pct.map(|w| -w),
                to_market_value_wan: None,
                from_rank: from_row.position_rank,
                to_rank: None,
            });
        }
    }

    sort_items(&mut items);
    items
}

/// 两侧均出现的股票：股数主口径判定加减仓，股数缺失时回退权重口径。
fn diff_matched(from_row: &PortfolioPosition, to_row: &PortfolioPosition) -> PositionDiffItem {
    let weight_delta_pp = match (from_row.weight_pct, to_row.weight_pct) {
        (Some(f), Some(t)) => Some(t - f),
        _ => None,
    };

    let (change_type, basis, shares_delta_wan, shares_delta_pct) =
        match (from_row.shares_wan, to_row.shares_wan) {
            (Some(f), Some(t)) => {
                let delta = t - f;
                let change = classify(delta, SHARES_EPSILON);
                let pct = if f.abs() > f64::EPSILON {
                    Some(delta / f * 100.0)
                } else {
                    None
                };
                (change, "shares", Some(delta), pct)
            }
            // 股数缺失（上游历史数据为 '---'）→ 回退权重口径。
            // 权重也不可比时无从判定，归为持平并保留权重口径标记。
            _ => {
                let change = match weight_delta_pp {
                    Some(dw) => classify(dw, WEIGHT_EPSILON),
                    None => "UNCHANGED",
                };
                (change, "weight", None, None)
            }
        };

    PositionDiffItem {
        stock_code: to_row.stock_code.clone(),
        stock_name: to_row.stock_name.clone(),
        change_type: change_type.to_string(),
        basis: basis.to_string(),
        from_shares_wan: from_row.shares_wan,
        to_shares_wan: to_row.shares_wan,
        shares_delta_wan,
        shares_delta_pct,
        from_weight_pct: from_row.weight_pct,
        to_weight_pct: to_row.weight_pct,
        weight_delta_pp,
        to_market_value_wan: to_row.market_value_wan,
        from_rank: from_row.position_rank,
        to_rank: to_row.position_rank,
    }
}

fn classify(delta: f64, epsilon: f64) -> &'static str {
    if delta.abs() <= epsilon {
        "UNCHANGED"
    } else if delta > 0.0 {
        "INCREASED"
    } else {
        "DECREASED"
    }
}

fn category_order(change_type: &str) -> u8 {
    match change_type {
        "NEW" => 0,
        "INCREASED" => 1,
        "DECREASED" => 2,
        "EXITED" => 3,
        _ => 4,
    }
}

fn sort_items(items: &mut [PositionDiffItem]) {
    items.sort_by(|a, b| {
        category_order(&a.change_type)
            .cmp(&category_order(&b.change_type))
            .then_with(|| {
                let wa = a.weight_delta_pp.map(f64::abs);
                let wb = b.weight_delta_pp.map(f64::abs);
                match (wa, wb) {
                    (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            })
            .then_with(|| a.stock_code.cmp(&b.stock_code))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PortfolioPosition;

    fn row(
        code: &str,
        name: &str,
        weight: Option<f64>,
        shares: Option<f64>,
        rank: i64,
    ) -> PortfolioPosition {
        PortfolioPosition {
            id: 0,
            portfolio_id: "p1".to_string(),
            as_of_date: "2025-12-31".to_string(),
            stock_code: code.to_string(),
            stock_name: name.to_string(),
            weight_pct: weight,
            shares_wan: shares,
            market_value_wan: shares.map(|s| s * 10.0),
            position_rank: Some(rank),
            created_at: "t".to_string(),
        }
    }

    fn find<'a>(items: &'a [PositionDiffItem], code: &str) -> &'a PositionDiffItem {
        items.iter().find(|i| i.stock_code == code).unwrap()
    }

    #[test]
    fn test_infer_coverage_march_september_always_partial() {
        // 3/9 月只有季报，即使行数很大也不可能是全量
        assert_eq!(infer_coverage("2026-03-31", 26), "PARTIAL");
        assert_eq!(infer_coverage("2025-09-30", 100), "PARTIAL");
    }

    #[test]
    fn test_infer_coverage_june_december_by_row_count() {
        assert_eq!(infer_coverage("2025-06-30", 34), "PARTIAL");
        assert_eq!(infer_coverage("2025-06-30", 35), "FULL");
        assert_eq!(infer_coverage("2025-12-31", 116), "FULL");
        assert_eq!(infer_coverage("2025-12-31", 12), "PARTIAL");
    }

    #[test]
    fn test_full_vs_full_all_categories() {
        // 全量对全量：新建 / 加仓 / 减仓 / 持平 / 清仓 各一例
        let from = vec![
            row("600519", "贵州茅台", Some(8.0), Some(100.0), 1),
            row("300750", "宁德时代", Some(5.0), Some(2000.0), 2),
            row("000568", "泸州老窖", Some(4.0), Some(1500.0), 3),
            row("601899", "紫金矿业", Some(3.0), Some(3000.0), 4),
        ];
        let to = vec![
            row("600519", "贵州茅台", Some(8.5), Some(150.0), 1),
            row("300750", "宁德时代", Some(4.0), Some(1200.0), 2),
            row("000568", "泸州老窖", Some(4.1), Some(1500.0), 3),
            row("688111", "金山办公", Some(2.0), Some(500.0), 4),
        ];
        let items = compute_diff(&from, &to);
        assert_eq!(items.len(), 5);

        let mao = find(&items, "600519");
        assert_eq!(mao.change_type, "INCREASED");
        assert_eq!(mao.basis, "shares");
        assert_eq!(mao.shares_delta_wan, Some(50.0));
        assert_eq!(mao.shares_delta_pct, Some(50.0));
        assert!((mao.weight_delta_pp.unwrap() - 0.5).abs() < 1e-9);

        let ning = find(&items, "300750");
        assert_eq!(ning.change_type, "DECREASED");
        assert_eq!(ning.shares_delta_pct, Some(-40.0));

        // 股数相同、权重微动 → 仍按股数判持平（权重不参与分类）
        let lu = find(&items, "000568");
        assert_eq!(lu.change_type, "UNCHANGED");
        assert_eq!(lu.basis, "shares");

        let jin = find(&items, "688111");
        assert_eq!(jin.change_type, "NEW");
        assert_eq!(jin.from_shares_wan, None);
        assert_eq!(jin.to_shares_wan, Some(500.0));
        assert_eq!(jin.weight_delta_pp, Some(2.0));

        let zi = find(&items, "601899");
        assert_eq!(zi.change_type, "EXITED");
        assert_eq!(zi.shares_delta_wan, Some(-3000.0));
        assert_eq!(zi.shares_delta_pct, Some(-100.0));
        assert_eq!(zi.weight_delta_pp, Some(-3.0));
    }

    #[test]
    fn test_category_and_weight_sorting() {
        let from = vec![
            row("111111", "小减", Some(2.0), Some(100.0), 1),
            row("222222", "大减", Some(5.0), Some(100.0), 2),
            row("333333", "清仓股", Some(1.0), Some(50.0), 3),
        ];
        let to = vec![
            row("111111", "小减", Some(1.8), Some(90.0), 1),
            row("222222", "大减", Some(2.0), Some(40.0), 2),
            row("444444", "新进股", Some(3.0), Some(200.0), 3),
        ];
        let items = compute_diff(&from, &to);
        let order: Vec<&str> = items.iter().map(|i| i.stock_code.as_str()).collect();
        // NEW → DECREASED（类内 |Δ权重| 降序：大减 3.0pp 在前）→ EXITED
        assert_eq!(order, vec!["444444", "222222", "111111", "333333"]);
    }

    #[test]
    fn test_shares_missing_falls_back_to_weight_basis() {
        // 旧数据股数为 '---'（None）→ 按权重口径判定并标记
        let from = vec![row("600036", "招商银行", Some(3.0), None, 1)];
        let to = vec![row("600036", "招商银行", Some(4.5), Some(2000.0), 1)];
        let items = compute_diff(&from, &to);
        assert_eq!(items[0].change_type, "INCREASED");
        assert_eq!(items[0].basis, "weight");
        assert_eq!(items[0].shares_delta_wan, None);
        assert!((items[0].weight_delta_pp.unwrap() - 1.5).abs() < 1e-9);
    }

    #[test]
    fn test_both_metrics_missing_is_unchanged() {
        let from = vec![row("600036", "招商银行", None, None, 1)];
        let to = vec![row("600036", "招商银行", None, None, 1)];
        let items = compute_diff(&from, &to);
        assert_eq!(items[0].change_type, "UNCHANGED");
        assert_eq!(items[0].basis, "weight");
    }

    #[test]
    fn test_hk_code_leading_zeros_preserved() {
        let from = vec![row("00700", "腾讯控股", Some(8.0), Some(456.6), 1)];
        let to = vec![
            row("00700", "腾讯控股", Some(8.2), Some(500.0), 1),
            row("01810", "小米集团-W", Some(3.0), Some(1000.0), 2),
        ];
        let items = compute_diff(&from, &to);
        assert_eq!(find(&items, "00700").change_type, "INCREASED");
        assert_eq!(find(&items, "01810").change_type, "NEW");
    }

    #[test]
    fn test_partial_disclosure_common_stock_compare_still_valid() {
        // 全量（100 行中的两只）对部分披露（前十大）：共同持有的股票
        // 股数对比不受口径影响——口径只改变展示措辞，不改变计算
        let from_full = vec![
            row("600160", "巨化股份", Some(1.2), Some(500.0), 30),
            row("688120", "华海清科", Some(6.0), Some(537.0), 2),
        ];
        let to_partial = vec![row("688120", "华海清科", Some(6.5), Some(600.0), 2)];
        let items = compute_diff(&from_full, &to_partial);
        let hua = find(&items, "688120");
        assert_eq!(hua.change_type, "INCREASED");
        assert_eq!(hua.basis, "shares");
        // 从全量消失的股票是 EXITED（中立事实）；"可能仍持有"的措辞由前端按口径渲染
        assert_eq!(find(&items, "600160").change_type, "EXITED");
    }

    #[test]
    fn test_epsilon_boundary() {
        let from = vec![row("600519", "贵州茅台", Some(8.0), Some(100.0), 1)];
        let to = vec![row("600519", "贵州茅台", Some(8.0), Some(100.0005), 1)];
        let items = compute_diff(&from, &to);
        assert_eq!(items[0].change_type, "UNCHANGED");
    }
}
