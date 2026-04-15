use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use crate::db::connection::Database;
use crate::db::queries::{warframes, abilities, weapons, mods};
use crate::fetcher::coordinator::{self, FetchReport, FetchProgress};
use crate::db::schema;
use rusqlite::Connection;
use std::path::Path;

#[derive(Serialize)]
pub struct DbStats {
    pub warframe_count: i64,
    pub ability_count: i64,
    pub weapon_count: i64,
    pub mod_count: i64,
}

#[tauri::command]
pub fn fetch_wiki_data(db: State<'_, Database>, app: AppHandle) -> Result<FetchReport, String> {
    let conn = Connection::open(&db.path).map_err(|e| format!("DB open error: {e}"))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| format!("PRAGMA error: {e}"))?;
    schema::create_tables(&conn).map_err(|e| format!("schema error: {e}"))?;

    let assets_dir = db.path.parent().unwrap_or(Path::new(".")).join("assets");

    let report = coordinator::fetch_all(&conn, &assets_dir, &|progress: FetchProgress| {
        let _ = app.emit("fetch_progress", &progress);
    });

    Ok(report)
}

#[tauri::command]
pub fn get_db_stats(db: State<'_, Database>) -> Result<DbStats, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(DbStats {
        warframe_count: warframes::get_warframe_count(&conn).map_err(|e| e.to_string())?,
        ability_count: abilities::get_ability_count(&conn).map_err(|e| e.to_string())?,
        weapon_count: weapons::get_weapon_count(&conn).map_err(|e| e.to_string())?,
        mod_count: mods::get_mod_count(&conn).map_err(|e| e.to_string())?,
    })
}
