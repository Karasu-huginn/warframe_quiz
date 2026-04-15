use rusqlite::{params, Connection};
use serde_json::Value;
use crate::fetcher::{CategoryResult, CategoryReport, ImageTask};
use crate::fetcher::wiki_client::WikiClient;
use crate::fetcher::lua_parser;

pub fn fetch_abilities(conn: &Connection, wiki: &WikiClient) -> Result<CategoryResult, String> {
    let source = wiki.fetch_module_source("Module:Ability/data")?;
    let data = lua_parser::eval_lua_module(&source)?;
    process_abilities_data(conn, &data)
}

pub fn process_abilities_data(conn: &Connection, data: &Value) -> Result<CategoryResult, String> {
    // The wiki module nests abilities under an "Ability" key (not "Warframe").
    // Also handle "Warframe" key or flat structure for backwards compatibility / tests.
    let entries = if let Some(section) = data.get("Ability").and_then(|v| v.as_object()) {
        section.clone()
    } else if let Some(section) = data.get("Warframe").and_then(|v| v.as_object()) {
        section.clone()
    } else {
        data.as_object().ok_or("abilities data: expected object")?.clone()
    };

    let mut report = CategoryReport { category: "abilities".to_string(), ..Default::default() };
    let mut images = Vec::new();

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    // Delete all existing abilities first, then re-insert fresh
    tx.execute("DELETE FROM abilities", []).map_err(|e| e.to_string())?;

    for (_key, entry) in &entries {
        let name = match entry["Name"].as_str() {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };

        let cost = entry["Cost"].as_i64().map(|v| v as i32).unwrap_or(0);
        let description = entry["Description"].as_str().unwrap_or_default();
        let icon_path = entry["Icon"].as_str()
            .map(|img| format!("assets/abilities/{img}"))
            .unwrap_or_default();
        let slot_index = entry["Key"].as_i64().map(|v| v as i32).unwrap_or(0);
        let is_helminth = entry["Subsumable"].as_bool().unwrap_or(false);
        let augment_mod_name = entry["Augment"].as_str().unwrap_or_default();
        let icon_filename = entry["Icon"].as_str().map(|s| s.to_string());

        // FK lookup: find warframe_id from PowerSuit name
        let powersuit = match entry["PowerSuit"].as_str() {
            Some(ps) if !ps.is_empty() => ps,
            _ => {
                // No PowerSuit field — skip
                report.failed += 1;
                continue;
            }
        };

        let warframe_id: Option<i64> = tx.query_row(
            "SELECT id FROM warframes WHERE name = ?1",
            params![powersuit],
            |row| row.get(0),
        ).ok();

        let warframe_id = match warframe_id {
            Some(id) => id,
            None => {
                eprintln!("Skipping ability '{name}': warframe '{powersuit}' not found in DB");
                report.failed += 1;
                continue;
            }
        };

        match tx.execute(
            "INSERT INTO abilities (name, cost, description, icon_path, warframe_id, slot_index, is_helminth, augment_mod_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![name, cost, description, icon_path, warframe_id, slot_index, is_helminth as i32, augment_mod_name],
        ) {
            Ok(_) => report.inserted += 1,
            Err(e) => {
                eprintln!("Failed to insert ability {name}: {e}");
                report.failed += 1;
            }
        }

        if let Some(img) = icon_filename {
            images.push(ImageTask { wiki_filename: img, local_subdir: "abilities".to_string() });
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

    fn insert_excalibur(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO warframes (name, type, description, passive, acquisition)
             VALUES ('Excalibur', 'Warframe', 'A balanced fighter', 'Swordsmanship', 'Starter')",
            [],
        ).unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn test_process_abilities_direct_root() {
        let conn = test_db();
        insert_excalibur(&conn);

        // Abilities directly at root level
        let data = lua_parser::eval_lua_module(r#"return {
            ["SlashDash"] = {
                Name = "Slash Dash", Cost = 25, Description = "Dash through enemies",
                Icon = "SlashDash.png", PowerSuit = "Excalibur", Key = 1,
                Subsumable = false, Augment = ""
            },
            ["RadialBlind"] = {
                Name = "Radial Blind", Cost = 50, Description = "Blind enemies",
                Icon = "RadialBlind.png", PowerSuit = "Excalibur", Key = 2,
                Subsumable = false, Augment = ""
            }
        }"#).unwrap();

        let result = process_abilities_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 2);
        assert_eq!(result.report.failed, 0);
        assert_eq!(result.images.len(), 2);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM abilities", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_process_abilities_nested_warframe_key() {
        let conn = test_db();
        insert_excalibur(&conn);

        // Abilities nested under "Warframe" key
        let data = lua_parser::eval_lua_module(r#"return {
            Warframe = {
                ["SlashDash"] = {
                    Name = "Slash Dash", Cost = 25, Description = "Dash through enemies",
                    Icon = "SlashDash.png", PowerSuit = "Excalibur", Key = 1,
                    Subsumable = false, Augment = ""
                }
            }
        }"#).unwrap();

        let result = process_abilities_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);
        assert_eq!(result.report.failed, 0);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM abilities", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_ability_missing_warframe_is_skipped() {
        let conn = test_db();
        // No warframes inserted — all abilities should fail FK lookup

        let data = lua_parser::eval_lua_module(r#"return {
            ["SlashDash"] = {
                Name = "Slash Dash", Cost = 25, Description = "Dash",
                Icon = "SlashDash.png", PowerSuit = "Excalibur", Key = 1,
                Subsumable = false, Augment = ""
            }
        }"#).unwrap();

        let result = process_abilities_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 0);
        assert_eq!(result.report.failed, 1);
    }

    #[test]
    fn test_abilities_deleted_on_reprocess() {
        let conn = test_db();
        insert_excalibur(&conn);

        let data = lua_parser::eval_lua_module(r#"return {
            ["SlashDash"] = {
                Name = "Slash Dash", Cost = 25, Description = "Dash",
                Icon = "SlashDash.png", PowerSuit = "Excalibur", Key = 1,
                Subsumable = false, Augment = ""
            }
        }"#).unwrap();

        process_abilities_data(&conn, &data).unwrap();
        // Process again — should delete + reinsert, not duplicate
        process_abilities_data(&conn, &data).unwrap();

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM abilities", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_ability_fk_stored_correctly() {
        let conn = test_db();
        let wf_id = insert_excalibur(&conn);

        let data = lua_parser::eval_lua_module(r#"return {
            ["SlashDash"] = {
                Name = "Slash Dash", Cost = 25, Description = "Dash",
                Icon = "SlashDash.png", PowerSuit = "Excalibur", Key = 1,
                Subsumable = true, Augment = "Slash Dash Augment"
            }
        }"#).unwrap();

        process_abilities_data(&conn, &data).unwrap();

        let (stored_wf_id, slot, is_helminth, augment): (i64, i32, i32, String) = conn.query_row(
            "SELECT warframe_id, slot_index, is_helminth, augment_mod_name FROM abilities WHERE name = 'Slash Dash'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        ).unwrap();

        assert_eq!(stored_wf_id, wf_id);
        assert_eq!(slot, 1);
        assert_eq!(is_helminth, 1);
        assert_eq!(augment, "Slash Dash Augment");
    }
}
