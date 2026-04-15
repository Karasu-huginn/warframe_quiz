use rusqlite::{params, Connection};
use serde_json::Value;
use crate::fetcher::{CategoryResult, CategoryReport, ImageTask};
use crate::fetcher::wiki_client::WikiClient;
use crate::fetcher::lua_parser;

pub fn fetch_warframes(conn: &Connection, wiki: &WikiClient) -> Result<CategoryResult, String> {
    let source = wiki.fetch_module_source("Module:Warframes/data")?;
    let data = lua_parser::eval_lua_module(&source)?;
    process_warframes_data(conn, &data)
}

pub fn process_warframes_data(conn: &Connection, data: &Value) -> Result<CategoryResult, String> {
    let root = data.as_object().ok_or("warframes data: expected object")?;
    let mut report = CategoryReport { category: "warframes".to_string(), ..Default::default() };
    let mut images = Vec::new();

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    // The wiki module nests entries under category keys: Warframe, Archwings, Necramechs, Operators
    // Merge all sub-tables, but also handle flat structure (for tests)
    let mut all_entries: Vec<&Value> = Vec::new();
    for (key, value) in root {
        if value.is_object() && value.get("Name").is_some() {
            // Flat structure: entry directly at root level
            all_entries.push(value);
        } else if let Some(sub_table) = value.as_object() {
            // Nested structure: sub-table containing entries
            for (_sub_key, entry) in sub_table {
                if entry.is_object() {
                    all_entries.push(entry);
                }
            }
        }
    }

    for entry in &all_entries {
        let name = match entry["Name"].as_str() {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };
        let wf_type = entry["Type"].as_str().unwrap_or("Warframe");
        let description = entry["Description"].as_str().unwrap_or_default();
        let health = entry["Health"].as_f64();
        let shields = entry["Shield"].as_f64();
        let armor = entry["Armor"].as_f64();
        let energy = entry["Energy"].as_f64();
        let sprint_speed = entry["Sprint"].as_f64();
        let passive = entry["Passive"].as_str().unwrap_or_default();
        let mastery_rank = entry["Mastery"].as_i64().map(|v| v as i32);
        let acquisition = entry["Acquisition"].as_str().unwrap_or_default();
        let release_date = entry["Introduced"].as_str().map(|s| s.to_string());
        let is_vaulted = entry["Vaulted"].as_bool().unwrap_or(false);
        let helminth_ability = entry["Subsumed"].as_str().map(|s| s.to_string());
        let sex = entry["Sex"].as_str().map(|s| s.to_string());
        let image = entry["Image"].as_str().map(|s| s.to_string());
        let icon_path = image.as_ref().map(|img| format!("assets/warframes/{img}"));

        match tx.execute(
            "INSERT INTO warframes (name, type, description, health, shields, armor, energy,
             sprint_speed, passive, mastery_rank, acquisition, release_date,
             is_vaulted, helminth_ability, sex, icon_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
             ON CONFLICT(name) DO UPDATE SET
             type=excluded.type, description=excluded.description, health=excluded.health,
             shields=excluded.shields, armor=excluded.armor, energy=excluded.energy,
             sprint_speed=excluded.sprint_speed, passive=excluded.passive,
             mastery_rank=excluded.mastery_rank, acquisition=excluded.acquisition,
             release_date=excluded.release_date, is_vaulted=excluded.is_vaulted,
             helminth_ability=excluded.helminth_ability, sex=excluded.sex,
             icon_path=excluded.icon_path",
            params![name, wf_type, description, health, shields, armor, energy,
                    sprint_speed, passive, mastery_rank, acquisition, release_date,
                    is_vaulted, helminth_ability, sex, icon_path],
        ) {
            Ok(_) => report.inserted += 1,
            Err(e) => {
                eprintln!("Failed to upsert warframe {name}: {e}");
                report.failed += 1;
            }
        }

        if let Some(img) = image {
            images.push(ImageTask { wiki_filename: img, local_subdir: "warframes".to_string() });
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(CategoryResult { report, images })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;
    use crate::fetcher::lua_parser;

    #[test]
    fn test_process_warframes() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Excalibur"] = {
                Name = "Excalibur", Type = "Warframe", Health = 100,
                Shield = 100, Armor = 225, Energy = 100, Sprint = 1.0,
                Passive = "Swordsmanship", Description = "A balanced fighter",
                Image = "Excalibur.png", Sex = "Male", Mastery = 0,
                Vaulted = false
            },
            ["Amesha"] = {
                Name = "Amesha", Type = "Archwing", Health = 100,
                Shield = 100, Armor = 50, Energy = 100, Sprint = 1.0,
                Passive = "", Description = "Support archwing",
                Image = "Amesha.png"
            }
        }"#).unwrap();

        let result = process_warframes_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 2);
        assert_eq!(result.report.failed, 0);
        assert_eq!(result.images.len(), 2);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM warframes", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 2);

        let name: String = conn.query_row(
            "SELECT name FROM warframes WHERE type = 'Archwing'", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(name, "Amesha");
    }

    #[test]
    fn test_upsert_updates_existing() {
        let conn = test_db();
        let data1 = lua_parser::eval_lua_module(r#"return {
            ["Excalibur"] = { Name = "Excalibur", Type = "Warframe", Health = 100 }
        }"#).unwrap();
        process_warframes_data(&conn, &data1).unwrap();

        let data2 = lua_parser::eval_lua_module(r#"return {
            ["Excalibur"] = { Name = "Excalibur", Type = "Warframe", Health = 370 }
        }"#).unwrap();
        process_warframes_data(&conn, &data2).unwrap();

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM warframes", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
        let health: f64 = conn.query_row("SELECT health FROM warframes WHERE name = 'Excalibur'", [], |r| r.get(0)).unwrap();
        assert_eq!(health, 370.0);
    }
}
