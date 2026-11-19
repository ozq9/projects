//! Token and cost accounting for every Claude call.

use chrono::{Datelike, Local, TimeZone, Utc};
use rusqlite::{params, Connection};
use serde::Serialize;

use crate::error::AppResult;

pub struct UsageRecord<'a> {
    pub purpose: &'a str,
    pub model: &'a str,
    pub item_id: Option<i64>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub cost_usd: Option<f64>,
}

pub fn record(conn: &Connection, r: &UsageRecord<'_>) -> AppResult<()> {
    conn.execute(
        "INSERT INTO ai_usage (purpose, model, item_id, input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens, cost_usd)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            r.purpose,
            r.model,
            r.item_id,
            r.input_tokens as i64,
            r.output_tokens as i64,
            r.cache_creation_tokens as i64,
            r.cache_read_tokens as i64,
            r.cost_usd
        ],
    )?;
    Ok(())
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageTotals {
    pub calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
    pub cost_usd: f64,
    /// Calls to models without a known price (not included in `cost_usd`).
    pub unpriced_calls: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageGroup {
    pub key: String,
    #[serde(flatten)]
    pub totals: UsageTotals,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub all_time: UsageTotals,
    pub this_month: UsageTotals,
    pub by_purpose: Vec<UsageGroup>,
    pub by_model: Vec<UsageGroup>,
}

const TOTALS_SELECT: &str = "COUNT(*), COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0),
    COALESCE(SUM(cache_creation_tokens),0), COALESCE(SUM(cache_read_tokens),0),
    COALESCE(SUM(cost_usd),0.0), COALESCE(SUM(cost_usd IS NULL),0)";

fn totals_from_row(row: &rusqlite::Row<'_>, offset: usize) -> rusqlite::Result<UsageTotals> {
    Ok(UsageTotals {
        calls: row.get(offset)?,
        input_tokens: row.get(offset + 1)?,
        output_tokens: row.get(offset + 2)?,
        cache_creation_tokens: row.get(offset + 3)?,
        cache_read_tokens: row.get(offset + 4)?,
        cost_usd: row.get(offset + 5)?,
        unpriced_calls: row.get(offset + 6)?,
    })
}

/// Start of the current month in the user's local time zone, as a UTC timestamp string.
fn local_month_start_iso() -> String {
    let now = Local::now();
    let start = Local
        .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .earliest()
        .unwrap_or(now);
    super::to_iso(start.with_timezone(&Utc))
}

fn grouped(conn: &Connection, column: &str) -> AppResult<Vec<UsageGroup>> {
    let sql = format!("SELECT {column}, {TOTALS_SELECT} FROM ai_usage GROUP BY {column} ORDER BY SUM(cost_usd) DESC, COUNT(*) DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| Ok(UsageGroup { key: row.get(0)?, totals: totals_from_row(row, 1)? }))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn summary(conn: &Connection) -> AppResult<UsageSummary> {
    let all_time = conn.query_row(&format!("SELECT {TOTALS_SELECT} FROM ai_usage"), [], |row| totals_from_row(row, 0))?;
    let this_month = conn.query_row(
        &format!("SELECT {TOTALS_SELECT} FROM ai_usage WHERE created_at >= ?1"),
        params![local_month_start_iso()],
        |row| totals_from_row(row, 0),
    )?;
    Ok(UsageSummary {
        all_time,
        this_month,
        by_purpose: grouped(conn, "purpose")?,
        by_model: grouped(conn, "model")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_summarises() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&mut conn).unwrap();
        let base = UsageRecord {
            purpose: "classify",
            model: "claude-opus-5-5",
            item_id: None,
            input_tokens: 1000,
            output_tokens: 200,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            cost_usd: Some(0.008),
        };
        record(&conn, &base).unwrap();
        record(&conn, &UsageRecord { purpose: "chat", model: "some-custom-model", cost_usd: None, ..base }).unwrap();

        let s = summary(&conn).unwrap();
        assert_eq!(s.all_time.calls, 2);
        assert_eq!(s.all_time.input_tokens, 2000);
        assert_eq!(s.all_time.unpriced_calls, 1);
        assert!((s.all_time.cost_usd - 0.008).abs() < 1e-9);
        assert_eq!(s.this_month.calls, 2);
        assert_eq!(s.by_purpose.len(), 2);
        assert_eq!(s.by_model[0].key, "claude-opus-5-5");
    }
}
