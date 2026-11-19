//! Minimal forward-only migrations tracked with `PRAGMA user_version`.
//! Add a new file under `src-tauri/migrations/` and append it to `MIGRATIONS`; never edit a
//! migration that has shipped.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

const MIGRATIONS: &[(&str, &str)] = &[("0001_init", include_str!("../../migrations/0001_init.sql"))];

pub fn latest_version() -> i64 {
    MIGRATIONS.len() as i64
}

pub fn current_version(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |row| row.get(0))?)
}

pub fn run(conn: &mut Connection) -> AppResult<()> {
    let current = current_version(conn)?;
    let latest = latest_version();
    if current > latest {
        return Err(AppError::invalid(format!(
            "This library was saved by a newer version of Satchel (schema {current}; this version understands up to {latest}). Update Satchel to open it."
        )));
    }
    for (index, (name, sql)) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (index + 1) as i64)?;
        tx.commit()?;
        tracing::info!(migration = name, "applied database migration");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_and_are_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        run(&mut conn).unwrap();
        assert_eq!(current_version(&conn).unwrap(), latest_version());
        run(&mut conn).unwrap(); // second run is a no-op

        // Spot-check the schema: cascade from items to result cards works.
        conn.execute("INSERT INTO items (source_kind) VALUES ('text')", []).unwrap();
        conn.execute("INSERT INTO result_cards (item_id, kind, content_json) VALUES (1, 'summary', '{}')", []).unwrap();
        conn.execute("DELETE FROM items WHERE id = 1", []).unwrap();
        let cards: i64 = conn.query_row("SELECT COUNT(*) FROM result_cards", [], |r| r.get(0)).unwrap();
        assert_eq!(cards, 0);
    }

    #[test]
    fn refuses_newer_schema() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", 999).unwrap();
        assert!(run(&mut conn).is_err());
    }
}
