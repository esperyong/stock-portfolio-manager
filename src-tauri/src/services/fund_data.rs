//! 天天基金/东方财富基金数据源。
//!
//! 两个非公开接口，均要求 `Referer` 头：
//! - 搜索：`fundsuggest.eastmoney.com/FundSearch/api/FundSearchAPI.ashx?m=1&key=<关键词>`
//!   返回 JSON（`Datas[].CODE / NAME / FundBaseInfo.FTYPE`）。
//! - 持仓明细：`fundf10.eastmoney.com/FundArchivesDatas.aspx?type=jjcc&code=<码>&topline=200&year=<年>`
//!   返回 `var apidata={ content:"<按季度的HTML表>",arryear:[...],curyear:...};`。
//!   每季度一张 `<div class='boxitem'>` 表，标题含「YYYY年N季度股票投资明细」；
//!   季报表固定 10 行，中报/年报发布后 Q2/Q4 表为全量持仓。
//!
//! HTML 表结构固定，用正则解析，不引入 HTML 解析依赖；上游改版时解析失败
//! 返回中文错误，不写脏数据（已落库的历史仓位不受影响）。

use crate::models::FundSearchResult;
use crate::services::http_client;
use regex::Regex;
use std::sync::OnceLock;

/// 单期（一个报告期）的持仓明细。
#[derive(Debug, Clone)]
pub struct FundPeriodHoldings {
    /// 报告期截止日（YYYY-MM-DD），由表标题的季度推断：
    /// Q1=03-31、Q2=06-30、Q3=09-30、Q4=12-31。
    pub as_of_date: String,
    pub rows: Vec<FundHoldingRow>,
}

/// 持仓表中的一行。
#[derive(Debug, Clone)]
pub struct FundHoldingRow {
    /// A股 6 位 / 港股 5 位代码，港股前导零原样保留。
    pub stock_code: String,
    pub stock_name: String,
    /// 占净值比例(%)。
    pub weight_pct: Option<f64>,
    /// 持股数（万股）。
    pub shares_wan: Option<f64>,
    /// 持仓市值（万元）。
    pub market_value_wan: Option<f64>,
    /// 表内排名。
    pub position_rank: Option<i64>,
}

fn fund_referer() -> &'static str {
    "https://fundf10.eastmoney.com/"
}

/// 调用 fundsuggest 联想搜索，返回候选基金列表（不落库）。
pub async fn search_funds(keyword: &str) -> Result<Vec<FundSearchResult>, String> {
    let resp = http_client::eastmoney_client()
        .get("https://fundsuggest.eastmoney.com/FundSearch/api/FundSearchAPI.ashx")
        .header(reqwest::header::REFERER, "https://fund.eastmoney.com/")
        .query(&[("m", "1"), ("key", keyword)])
        .send()
        .await
        .map_err(|e| format!("基金搜索请求失败：{}", e))?;
    let body = resp
        .text()
        .await
        .map_err(|e| format!("基金搜索响应读取失败：{}", e))?;
    parse_fund_search_response(&body)
}

/// 解析 fundsuggest 的 JSON 响应。只保留携带 `FundBaseInfo` 的条目
/// （其余为股票/组合等非基金类目）。
pub fn parse_fund_search_response(body: &str) -> Result<Vec<FundSearchResult>, String> {
    let json: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| "基金搜索响应格式异常，接口可能已变更".to_string())?;
    let mut results = Vec::new();
    let datas = match json.get("Datas").and_then(|v| v.as_array()) {
        Some(d) => d,
        None => return Ok(results),
    };
    for item in datas {
        let base = match item.get("FundBaseInfo") {
            Some(b) if b.is_object() => b,
            _ => continue,
        };
        let code = item.get("CODE").and_then(|v| v.as_str()).unwrap_or("");
        let name = item.get("NAME").and_then(|v| v.as_str()).unwrap_or("");
        if code.is_empty() || name.is_empty() {
            continue;
        }
        let fund_type = base
            .get("FTYPE")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        results.push(FundSearchResult {
            fund_code: code.to_string(),
            fund_name: name.to_string(),
            fund_type,
        });
    }
    Ok(results)
}

/// 抓取基金当年全部已披露期次的持仓；当年无任何期次（年初定期报告未出）
/// 时自动回退抓取上一年。
pub async fn fetch_fund_holdings(fund_code: &str) -> Result<Vec<FundPeriodHoldings>, String> {
    let current_year: i32 = chrono::Utc::now()
        .format("%Y")
        .to_string()
        .parse()
        .map_err(|_| "获取当前年份失败".to_string())?;
    let periods = fetch_fund_holdings_for_year(fund_code, current_year).await?;
    if !periods.is_empty() {
        return Ok(periods);
    }
    let periods = fetch_fund_holdings_for_year(fund_code, current_year - 1).await?;
    if periods.is_empty() {
        return Err("未获取到该基金的股票持仓数据（当年与上一年均无披露）".to_string());
    }
    Ok(periods)
}

async fn fetch_fund_holdings_for_year(
    fund_code: &str,
    year: i32,
) -> Result<Vec<FundPeriodHoldings>, String> {
    let resp = http_client::eastmoney_client()
        .get("https://fundf10.eastmoney.com/FundArchivesDatas.aspx")
        .header(reqwest::header::REFERER, fund_referer())
        .query(&[
            ("type", "jjcc"),
            ("code", fund_code),
            ("topline", "200"),
            ("year", &year.to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("基金持仓请求失败：{}", e))?;
    let body = resp
        .text()
        .await
        .map_err(|e| format!("基金持仓响应读取失败：{}", e))?;
    parse_jjcc_response(&body)
}

static CONTENT_RE: OnceLock<Regex> = OnceLock::new();
static TITLE_RE: OnceLock<Regex> = OnceLock::new();
static TR_RE: OnceLock<Regex> = OnceLock::new();
static TD_RE: OnceLock<Regex> = OnceLock::new();
static TAG_RE: OnceLock<Regex> = OnceLock::new();
static CODE_RE: OnceLock<Regex> = OnceLock::new();

/// 解析 jjcc 响应，返回该年全部期次（可能为空——该年无披露）。
/// 响应含表却一行都解析不出时视为上游改版，返回中文错误。
pub fn parse_jjcc_response(body: &str) -> Result<Vec<FundPeriodHoldings>, String> {
    let content_re = CONTENT_RE.get_or_init(|| {
        Regex::new(r#"(?s)content:"(.*)"\s*,\s*arryear\s*:"#).unwrap()
    });
    let title_re = TITLE_RE
        .get_or_init(|| Regex::new(r"(\d{4})年(\d)季度股票投资明细").unwrap());

    let content = match content_re.captures(body) {
        Some(caps) => caps.get(1).map(|m| m.as_str()).unwrap_or(""),
        None => return Err("基金持仓响应格式异常，接口可能已变更".to_string()),
    };
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut periods = Vec::new();
    // 每个报告期一个 <div class='boxitem'> 区块，标题在区块内。
    for section in content.split("<div class='boxitem").skip(1) {
        let caps = match title_re.captures(section) {
            Some(c) => c,
            None => continue,
        };
        let as_of_date = match quarter_end_date(&caps[1], &caps[2]) {
            Some(d) => d,
            None => continue,
        };
        let rows = parse_holding_rows(section);
        if rows.is_empty() {
            continue;
        }
        periods.push(FundPeriodHoldings { as_of_date, rows });
    }

    // 内容里明确存在持仓表标题却一期都没解析出来 → 表结构改版。
    if periods.is_empty() && title_re.is_match(content) {
        return Err("解析基金持仓表失败，上游页面结构可能已变更".to_string());
    }
    Ok(periods)
}

/// 表标题的季度 → 报告期截止日。
fn quarter_end_date(year: &str, quarter: &str) -> Option<String> {
    let month_day = match quarter {
        "1" => "03-31",
        "2" => "06-30",
        "3" => "09-30",
        "4" => "12-31",
        _ => return None,
    };
    Some(format!("{}-{}", year, month_day))
}

/// 逐行解析一个报告期区块内的持仓表。
///
/// 行结构（当年表）：序号 | 代码 | 名称 | 最新价 | 涨跌幅 | 相关资讯 | 占净值比例 | 持股数 | 持仓市值。
/// 历史年份表没有最新价/涨跌幅列，因此按语义取列而非固定下标：
/// 首个以 `%` 结尾的单元格为权重，其后两个数值单元格依次为持股数与市值。
/// 全量持仓表中超出前十大的行序号带 `*` 标记（如 `11*`），解析时剥离。
fn parse_holding_rows(section: &str) -> Vec<FundHoldingRow> {
    let tr_re = TR_RE.get_or_init(|| Regex::new(r"(?s)<tr[^>]*>(.*?)</tr>").unwrap());
    let td_re = TD_RE.get_or_init(|| Regex::new(r"(?s)<td[^>]*>(.*?)</td>").unwrap());
    let tag_re = TAG_RE.get_or_init(|| Regex::new(r"<[^>]+>").unwrap());
    let code_re = CODE_RE.get_or_init(|| Regex::new(r"^\d{5,6}$").unwrap());

    let mut rows = Vec::new();
    for tr in tr_re.captures_iter(section) {
        let cells: Vec<String> = td_re
            .captures_iter(&tr[1])
            .map(|c| {
                tag_re
                    .replace_all(&c[1], "")
                    .replace("&nbsp;", " ")
                    .trim()
                    .to_string()
            })
            .collect();
        // 表头行（<th>）没有 <td>，无效行直接跳过。
        if cells.len() < 3 {
            continue;
        }
        let rank: Option<i64> = cells[0].trim_end_matches('*').parse().ok();
        if rank.is_none() {
            continue;
        }
        let stock_code = cells[1].trim();
        if !code_re.is_match(stock_code) {
            continue;
        }
        let stock_name = cells[2].trim_end_matches('*').trim().to_string();
        if stock_name.is_empty() {
            continue;
        }

        let mut weight_pct = None;
        let mut trailing_nums: Vec<f64> = Vec::new();
        for cell in &cells[3..] {
            if cell.is_empty() || cell == "---" || cell == "--" {
                continue;
            }
            if weight_pct.is_none() {
                // 权重列之前的单元格（最新价/涨跌幅为空 span，相关资讯为链接文字）一律忽略。
                if let Some(stripped) = cell.strip_suffix('%') {
                    weight_pct = parse_number(stripped);
                }
                continue;
            }
            if trailing_nums.len() < 2 {
                if let Some(v) = parse_number(cell) {
                    trailing_nums.push(v);
                }
            }
        }

        rows.push(FundHoldingRow {
            stock_code: stock_code.to_string(),
            stock_name,
            weight_pct,
            shares_wan: trailing_nums.first().copied(),
            market_value_wan: trailing_nums.get(1).copied(),
            position_rank: rank,
        });
    }
    rows
}

/// 解析带千分位的数值，如 `124,542.98`。
fn parse_number(s: &str) -> Option<f64> {
    s.trim().replace(',', "").parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 基于真实响应（163415，2026-07 抓取）裁剪的季报 10 行表片段。
    fn quarterly_fixture() -> String {
        let mut rows = String::new();
        let data = [
            ("1", "600160", "巨化股份", "6.57%", "2,925.08", "99,891.48"),
            ("2", "688120", "华海清科", "6.15%", "537.73", "93,564.46"),
            ("3", "605117", "德业股份", "5.90%", "1,050.00", "89,000.10"),
            ("4", "300502", "新易盛", "5.52%", "337.66", "84,542.98"),
            ("5", "600519", "贵州茅台", "5.10%", "50.00", "80,000.00"),
            ("6", "000568", "泸州老窖", "4.80%", "600.00", "75,321.00"),
            ("7", "002475", "立讯精密", "4.20%", "1,800.00", "70,555.55"),
            ("8", "601899", "紫金矿业", "3.90%", "3,200.00", "65,432.10"),
            ("9", "688111", "金山办公", "3.50%", "180.00", "60,123.45"),
            ("10", "300750", "宁德时代", "3.10%", "250.00", "55,000.00"),
        ];
        for (rank, code, name, weight, shares, value) in data {
            rows.push_str(&format!(
                "<tr><td>{rank}</td><td><a href='//quote.eastmoney.com/unify/r/1.{code}'>{code}</a></td>\
                 <td class='tol'><a href='//quote.eastmoney.com/unify/r/1.{code}'>{name}</a></td>\
                 <td class='tor'><span data-id='dq{code}'></span></td>\
                 <td class='tor'><span data-id='zd{code}'></span></td>\
                 <td class='xglj'><a href='ccbdxq_163415_{code}.html' class='red'>变动详情</a>\
                 <a href='//guba.eastmoney.com/interface/GetList.aspx?code=1.{code}' >股吧</a>\
                 <a href='//quote.eastmoney.com/unify/r/1.{code}' >行情</a></td>\
                 <td class='tor'>{weight}</td><td class='tor'>{shares}</td><td class='tor'>{value}</td></tr>"
            ));
        }
        format!(
            "var apidata={{ content:\"<div class='box'><div class='boxitem w790'><h4 class='t'>\
             <label class='left'><a title='兴全商业模式混合(LOF)A' href='http://fund.eastmoney.com/163415.html'>\
             兴全商业模式混合(LOF)A</a>&nbsp;&nbsp;2026年1季度股票投资明细</label>\
             <label class='right lab2 xq505'>&nbsp;&nbsp;来源：天天基金&nbsp;&nbsp;截止至：<font class='px12'>2026-03-31</font></label></h4>\
             <div class='space0'></div><table class='w782 comm tzxq'><thead><tr><th class='first'>序号</th>\
             <th>股票代码</th><th>股票名称</th><th>最新价</th><th>涨跌幅</th><th class='xglj'>相关资讯</th>\
             <th>占净值<br />比例</th><th class='cgs'>持股数<br />（万股）</th><th class='last ccs'>持仓市值<br />（万元）</th></tr></thead>\
             <tbody>{rows}</tbody></table></div></div>\",arryear:[2026,2025,2024],curyear:2026}};"
        )
    }

    /// 年报全量表：前十大之后的行序号带 `*` 标记；同响应含两个报告期区块。
    /// 含港股通标的行（5 位代码，前导零保留）。
    fn annual_full_fixture() -> String {
        let q4_rows = "\
            <tr><td>1</td><td><a href='//quote.eastmoney.com/unify/r/116.00700'>00700</a></td>\
            <td class='tol'><a href='//quote.eastmoney.com/unify/r/116.00700'>腾讯控股</a></td>\
            <td class='xglj'><a href='ccbdxq_163417_00700.html' class='red'>变动详情</a></td>\
            <td class='tor'>8.11%</td><td class='tor'>456.60</td><td class='tor'>170,996.44</td></tr>\
            <tr><td>2</td><td><a href='//quote.eastmoney.com/unify/r/1.600519'>600519</a></td>\
            <td class='tol'><a href='//quote.eastmoney.com/unify/r/1.600519'>贵州茅台</a></td>\
            <td class='xglj'><a href='ccbdxq_163417_600519.html' class='red'>变动详情</a></td>\
            <td class='tor'>7.90%</td><td class='tor'>80.00</td><td class='tor'>150,000.00</td></tr>\
            <tr><td>11*</td><td><a href='//quote.eastmoney.com/unify/r/116.00981'>00981</a></td>\
            <td class='tol'><a href='//quote.eastmoney.com/unify/r/116.00981'>中芯国际</a></td>\
            <td class='xglj'><a href='ccbdxq_163417_00981.html' class='red'>变动详情</a></td>\
            <td class='tor'>1.95%</td><td class='tor'>320.00</td><td class='tor'>15,432.10</td></tr>\
            <tr><td>12*</td><td><a href='//quote.eastmoney.com/unify/r/0.000333'>000333</a></td>\
            <td class='tol'><a href='//quote.eastmoney.com/unify/r/0.000333'>美的集团</a></td>\
            <td class='xglj'><a href='ccbdxq_163417_000333.html' class='red'>变动详情</a></td>\
            <td class='tor'>1.62%</td><td class='tor'>210.55</td><td class='tor'>12,001.23</td></tr>";
        let q3_rows = "\
            <tr><td>1</td><td><a href='//quote.eastmoney.com/unify/r/1.600036'>600036</a></td>\
            <td class='tol'><a href='//quote.eastmoney.com/unify/r/1.600036'>招商银行</a></td>\
            <td class='xglj'><a href='ccbdxq_163417_600036.html' class='red'>变动详情</a></td>\
            <td class='tor'>9.00%</td><td class='tor'>2,000.00</td><td class='tor'>88,888.88</td></tr>";
        format!(
            "var apidata={{ content:\"<div class='box'><div class='boxitem w790'><h4 class='t'>\
             <label class='left'>兴全合宜混合(LOF)A&nbsp;&nbsp;2025年4季度股票投资明细</label></h4>\
             <table class='w782 comm tzxq'><thead><tr><th>序号</th><th>股票代码</th><th>股票名称</th>\
             <th class='xglj'>相关资讯</th><th>占净值比例</th><th>持股数（万股）</th><th>持仓市值（万元）</th></tr></thead>\
             <tbody>{q4_rows}</tbody></table></div>\
             <div class='boxitem w790'><h4 class='t'>\
             <label class='left'>兴全合宜混合(LOF)A&nbsp;&nbsp;2025年3季度股票投资明细</label></h4>\
             <table class='w782 comm tzxq'><thead><tr><th>序号</th><th>股票代码</th><th>股票名称</th>\
             <th class='xglj'>相关资讯</th><th>占净值比例</th><th>持股数（万股）</th><th>持仓市值（万元）</th></tr></thead>\
             <tbody>{q3_rows}</tbody></table></div></div>\",arryear:[2025,2024],curyear:2025}};"
        )
    }

    #[test]
    fn test_parse_quarterly_ten_row_table() {
        let periods = parse_jjcc_response(&quarterly_fixture()).unwrap();
        assert_eq!(periods.len(), 1);
        let period = &periods[0];
        assert_eq!(period.as_of_date, "2026-03-31");
        assert_eq!(period.rows.len(), 10);

        let first = &period.rows[0];
        assert_eq!(first.stock_code, "600160");
        assert_eq!(first.stock_name, "巨化股份");
        assert_eq!(first.weight_pct, Some(6.57));
        assert_eq!(first.shares_wan, Some(2925.08));
        assert_eq!(first.market_value_wan, Some(99891.48));
        assert_eq!(first.position_rank, Some(1));

        let last = &period.rows[9];
        assert_eq!(last.stock_code, "300750");
        assert_eq!(last.position_rank, Some(10));
    }

    #[test]
    fn test_parse_annual_full_table_with_starred_ranks_and_multiple_periods() {
        let periods = parse_jjcc_response(&annual_full_fixture()).unwrap();
        assert_eq!(periods.len(), 2);

        let q4 = &periods[0];
        assert_eq!(q4.as_of_date, "2025-12-31");
        assert_eq!(q4.rows.len(), 4);
        // 全量表中超出前十大的行（序号带 *）也要解析出来。
        assert_eq!(q4.rows[2].position_rank, Some(11));
        assert_eq!(q4.rows[2].stock_name, "中芯国际");
        assert_eq!(q4.rows[3].position_rank, Some(12));
        assert_eq!(q4.rows[3].stock_code, "000333");

        let q3 = &periods[1];
        assert_eq!(q3.as_of_date, "2025-09-30");
        assert_eq!(q3.rows.len(), 1);
        assert_eq!(q3.rows[0].stock_code, "600036");
        assert_eq!(q3.rows[0].shares_wan, Some(2000.0));
    }

    #[test]
    fn test_parse_hk_code_keeps_leading_zeros() {
        let periods = parse_jjcc_response(&annual_full_fixture()).unwrap();
        let q4 = &periods[0];
        assert_eq!(q4.rows[0].stock_code, "00700");
        assert_eq!(q4.rows[0].stock_name, "腾讯控股");
        assert_eq!(q4.rows[0].weight_pct, Some(8.11));
        assert_eq!(q4.rows[2].stock_code, "00981");
    }

    #[test]
    fn test_parse_empty_response_returns_no_periods() {
        // 该年无任何披露（真实响应格式，如货币基金或年初未出季报）。
        let body = r#"var apidata={ content:"",arryear:[],curyear:2026};"#;
        let periods = parse_jjcc_response(body).unwrap();
        assert!(periods.is_empty());
    }

    #[test]
    fn test_parse_garbage_response_is_error() {
        assert!(parse_jjcc_response("<html>404 not found</html>").is_err());
    }

    #[test]
    fn test_parse_changed_table_layout_is_error() {
        // 有报告期标题但表行结构无法解析 → 报错而非静默返回空。
        let body = "var apidata={ content:\"<div class='boxitem'><h4>2025年4季度股票投资明细</h4>\
                    <table><ul><li>600519 贵州茅台</li></ul></table></div>\",arryear:[2025],curyear:2025};";
        assert!(parse_jjcc_response(body).is_err());
    }

    #[test]
    fn test_parse_fund_search_response() {
        // 真实响应（2026-07 抓取）裁剪：基金条目带 FundBaseInfo，股票等其他类目为 null。
        let body = r#"{"ErrCode":0,"ErrMsg":"fromes","Datas":[
            {"CODE":"163415","NAME":"兴全商业模式混合(LOF)A","CATEGORYDESC":"基金",
             "FundBaseInfo":{"FCODE":"163415","FTYPE":"混合型-偏股","SHORTNAME":"兴全商业模式混合(LOF)A"}},
            {"CODE":"005491","NAME":"兴全合宜混合C","CATEGORYDESC":"基金",
             "FundBaseInfo":{"FCODE":"005491","FTYPE":"混合型-偏股","SHORTNAME":"兴全合宜混合C"}},
            {"CODE":"600519","NAME":"贵州茅台","CATEGORYDESC":"AB股","FundBaseInfo":null}
        ]}"#;
        let results = parse_fund_search_response(body).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].fund_code, "163415");
        assert_eq!(results[0].fund_type, "混合型-偏股");
        assert_eq!(results[1].fund_code, "005491");
        assert_eq!(results[1].fund_name, "兴全合宜混合C");
    }

    #[test]
    fn test_parse_fund_search_bad_json_is_error() {
        assert!(parse_fund_search_response("<html>blocked</html>").is_err());
    }

    #[test]
    fn test_quarter_end_date_mapping() {
        assert_eq!(quarter_end_date("2025", "1").as_deref(), Some("2025-03-31"));
        assert_eq!(quarter_end_date("2025", "2").as_deref(), Some("2025-06-30"));
        assert_eq!(quarter_end_date("2025", "3").as_deref(), Some("2025-09-30"));
        assert_eq!(quarter_end_date("2025", "4").as_deref(), Some("2025-12-31"));
        assert_eq!(quarter_end_date("2025", "5"), None);
    }
}
