use serde::Serialize;
use tauri::State;
use crate::db::connection::Database;
use crate::db::queries::{warframes, abilities, weapons, mods, characters, quotes};

#[derive(Serialize)]
pub struct DbStats {
    pub warframe_count: i64,
    pub ability_count: i64,
    pub weapon_count: i64,
    pub mod_count: i64,
    pub character_count: i64,
    pub quote_count: i64,
}

#[tauri::command]
pub fn get_db_stats(db: State<'_, Database>) -> Result<DbStats, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(DbStats {
        warframe_count: warframes::get_warframe_count(&conn).map_err(|e| e.to_string())?,
        ability_count: abilities::get_ability_count(&conn).map_err(|e| e.to_string())?,
        weapon_count: weapons::get_weapon_count(&conn).map_err(|e| e.to_string())?,
        mod_count: mods::get_mod_count(&conn).map_err(|e| e.to_string())?,
        character_count: characters::get_character_count(&conn).map_err(|e| e.to_string())?,
        quote_count: quotes::get_quote_count(&conn).map_err(|e| e.to_string())?,
    })
}
