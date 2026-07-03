//! 基金跟踪：组合 + 仓位的 IPC 命令。
//!
//! 组合域与真实资产域（accounts/holdings/transactions）零外键耦合，
//! 不进入总资产/收益等下游链路。网络抓取（async）一律先完成再锁库，
//! 绝不跨 `.await` 持有 DB mutex。

use crate::db::Database;
use crate::models::{FundSearchResult, Portfolio, PortfolioPosition};
use crate::services::fund_data;
use tauri::State;

/// 基金联想搜索（不落库）。
#[tauri::command(rename_all = "camelCase")]
pub async fn search_funds(keyword: String) -> Result<Vec<FundSearchResult>, String> {
    let keyword = keyword.trim().to_string();
    if keyword.is_empty() {
        return Ok(Vec::new());
    }
    fund_data::search_funds(&keyword).await
}

/// 从搜索候选创建基金组合；同一基金代码不可重复跟踪。
/// 创建成功后自动做一次首刷；首刷失败时组合仍保留（可稍后手动刷新）。
#[tauri::command(rename_all = "camelCase")]
pub async fn create_fund_portfolio(
    db: State<'_, Database>,
    fund_code: String,
    fund_name: String,
    fund_type: Option<String>,
) -> Result<Portfolio, String> {
    let fund_code = fund_code.trim().to_string();
    if fund_code.is_empty() || fund_name.trim().is_empty() {
        return Err("基金代码与名称不能为空".to_string());
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM portfolios WHERE fund_code = ?1",
                rusqlite::params![fund_code],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if exists > 0 {
            return Err("该基金已在跟踪列表中".to_string());
        }
        conn.execute(
            "INSERT INTO portfolios (id, name, source_type, fund_code, fund_type, created_at, updated_at)
             VALUES (?1, ?2, 'FUND', ?3, ?4, ?5, ?5)",
            rusqlite::params![id, fund_name.trim(), fund_code, fund_type, now],
        )
        .map_err(|e| e.to_string())?;
    } // 锁在网络请求前释放

    // 自动首刷：失败不回滚组合（数据可稍后手动刷新补回）。
    let refresh_error = match refresh_portfolio_impl(&db, &id).await {
        Ok(_) => None,
        Err(e) => Some(e),
    };

    let portfolio = load_portfolio(&db, &id)?;
    if let Some(e) = refresh_error {
        eprintln!("Fund portfolio {} first refresh failed: {}", fund_code, e);
        return Err(format!("组合已创建，但首次刷新失败：{}（可稍后点击「刷新」重试）", e));
    }
    Ok(portfolio)
}

/// 删除组合，级联删除其全部仓位版本（基金历史可随时从接口回补）。
#[tauri::command(rename_all = "camelCase")]
pub fn delete_portfolio(db: State<Database>, portfolio_id: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let rows = conn
        .execute(
            "DELETE FROM portfolios WHERE id = ?1",
            rusqlite::params![portfolio_id],
        )
        .map_err(|e| e.to_string())?;
    if rows == 0 {
        return Err("未找到该组合".to_string());
    }
    Ok(())
}

/// 组合列表，附带每个组合最新一期的 `as_of_date`。
#[tauri::command(rename_all = "camelCase")]
pub fn list_portfolios(db: State<Database>) -> Result<Vec<Portfolio>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.name, p.source_type, p.fund_code, p.fund_type,
                    (SELECT MAX(pp.as_of_date) FROM portfolio_positions pp WHERE pp.portfolio_id = p.id),
                    p.last_refreshed_at, p.created_at, p.updated_at
             FROM portfolios p
             ORDER BY p.created_at",
        )
        .map_err(|e| e.to_string())?;
    let portfolios = stmt
        .query_map([], map_portfolio_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(portfolios)
}

/// 手动刷新：抓取该基金当年（无则回退上一年）全部期次，幂等 upsert 后
/// 返回最新一期仓位。
#[tauri::command(rename_all = "camelCase")]
pub async fn refresh_fund_portfolio(
    db: State<'_, Database>,
    portfolio_id: String,
) -> Result<Vec<PortfolioPosition>, String> {
    refresh_portfolio_impl(&db, &portfolio_id).await
}

/// 只读库：返回组合最新一期（`as_of_date` 最大）的全部仓位行，按权重降序。
#[tauri::command(rename_all = "camelCase")]
pub fn get_portfolio_positions(
    db: State<Database>,
    portfolio_id: String,
) -> Result<Vec<PortfolioPosition>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    query_latest_positions(&conn, &portfolio_id)
}

/// 刷新的共用实现（供手动刷新与创建后的首刷调用）。
/// 网络抓取与解析在无锁状态下完成，落库阶段才短暂加锁。
async fn refresh_portfolio_impl(
    db: &Database,
    portfolio_id: &str,
) -> Result<Vec<PortfolioPosition>, String> {
    let fund_code: String = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT fund_code FROM portfolios WHERE id = ?1 AND source_type = 'FUND'",
            rusqlite::params![portfolio_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .map_err(|_| "未找到该基金组合".to_string())?
        .ok_or_else(|| "该组合缺少基金代码，无法刷新".to_string())?
    }; // 锁在网络请求前释放

    let periods = fund_data::fetch_fund_holdings(&fund_code).await?;

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    for period in &periods {
        for row in &period.rows {
            conn.execute(
                "INSERT INTO portfolio_positions
                     (portfolio_id, as_of_date, stock_code, stock_name,
                      weight_pct, shares_wan, market_value_wan, position_rank, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(portfolio_id, as_of_date, stock_code) DO UPDATE SET
                     stock_name = excluded.stock_name,
                     weight_pct = excluded.weight_pct,
                     shares_wan = excluded.shares_wan,
                     market_value_wan = excluded.market_value_wan,
                     position_rank = excluded.position_rank",
                rusqlite::params![
                    portfolio_id,
                    period.as_of_date,
                    row.stock_code,
                    row.stock_name,
                    row.weight_pct,
                    row.shares_wan,
                    row.market_value_wan,
                    row.position_rank,
                    now
                ],
            )
            .map_err(|e| format!("仓位数据写入失败：{}", e))?;
        }
    }
    conn.execute(
        "UPDATE portfolios SET last_refreshed_at = ?2, updated_at = ?2 WHERE id = ?1",
        rusqlite::params![portfolio_id, now],
    )
    .map_err(|e| e.to_string())?;

    query_latest_positions(&conn, portfolio_id)
}

fn load_portfolio(db: &Database, id: &str) -> Result<Portfolio, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT p.id, p.name, p.source_type, p.fund_code, p.fund_type,
                (SELECT MAX(pp.as_of_date) FROM portfolio_positions pp WHERE pp.portfolio_id = p.id),
                p.last_refreshed_at, p.created_at, p.updated_at
         FROM portfolios p WHERE p.id = ?1",
        rusqlite::params![id],
        map_portfolio_row,
    )
    .map_err(|_| "未找到该组合".to_string())
}

fn map_portfolio_row(row: &rusqlite::Row) -> rusqlite::Result<Portfolio> {
    Ok(Portfolio {
        id: row.get(0)?,
        name: row.get(1)?,
        source_type: row.get(2)?,
        fund_code: row.get(3)?,
        fund_type: row.get(4)?,
        latest_as_of_date: row.get(5)?,
        last_refreshed_at: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn query_latest_positions(
    conn: &rusqlite::Connection,
    portfolio_id: &str,
) -> Result<Vec<PortfolioPosition>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, portfolio_id, as_of_date, stock_code, stock_name,
                    weight_pct, shares_wan, market_value_wan, position_rank, created_at
             FROM portfolio_positions
             WHERE portfolio_id = ?1
               AND as_of_date = (SELECT MAX(as_of_date) FROM portfolio_positions WHERE portfolio_id = ?1)
             ORDER BY weight_pct DESC",
        )
        .map_err(|e| e.to_string())?;
    let positions = stmt
        .query_map(rusqlite::params![portfolio_id], |row| {
            Ok(PortfolioPosition {
                id: row.get(0)?,
                portfolio_id: row.get(1)?,
                as_of_date: row.get(2)?,
                stock_code: row.get(3)?,
                stock_name: row.get(4)?,
                weight_pct: row.get(5)?,
                shares_wan: row.get(6)?,
                market_value_wan: row.get(7)?,
                position_rank: row.get(8)?,
                created_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(positions)
}
