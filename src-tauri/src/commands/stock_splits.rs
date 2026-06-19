use crate::db::Database;
use crate::models::stock_split::StockSplit;
use tauri::State;

#[tauri::command(rename_all = "camelCase")]
pub fn get_stock_splits(db: State<'_, Database>) -> Result<Vec<StockSplit>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, stock_code, split_date, ratio_from, ratio_to, created_at
             FROM stock_splits
             ORDER BY split_date DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(StockSplit {
                id: row.get(0)?,
                stock_code: row.get(1)?,
                split_date: row.get(2)?,
                ratio_from: row.get(3)?,
                ratio_to: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut splits = Vec::new();
    for row in rows {
        splits.push(row.map_err(|e| e.to_string())?);
    }
    Ok(splits)
}

#[tauri::command(rename_all = "camelCase")]
pub fn add_stock_split(
    db: State<'_, Database>,
    stock_code: String,
    split_date: String,
    ratio_from: i64,
    ratio_to: i64,
) -> Result<StockSplit, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO stock_splits (stock_code, split_date, ratio_from, ratio_to, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![stock_code, split_date, ratio_from, ratio_to, now],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    Ok(StockSplit {
        id,
        stock_code,
        split_date,
        ratio_from,
        ratio_to,
        created_at: now,
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn delete_stock_split(db: State<'_, Database>, id: i64) -> Result<bool, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let rows = conn
        .execute("DELETE FROM stock_splits WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    Ok(rows > 0)
}
