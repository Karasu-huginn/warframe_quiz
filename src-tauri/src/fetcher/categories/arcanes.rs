use rusqlite::{params, Connection};
use serde_json::Value;
use crate::fetcher::{CategoryResult, CategoryReport, ImageTask};
use crate::fetcher::wiki_client::WikiClient;
use crate::fetcher::lua_parser;

pub fn fetch_arcanes(conn: &Connection, wiki: &WikiClient) -> Result<CategoryResult, String> {
    let source = wiki.fetch_module_source("Module:Arcane/data")?;
    let data = lua_parser::eval_lua_module(&source)?;
    process_arcanes_data(conn, &data)
}

pub fn process_arcanes_data(conn: &Connection, data: &Value) -> Result<CategoryResult, String> {
    let entries = data.as_object().ok_or("arcanes data: expected object")?;
    let mut report = CategoryReport { category: "arcanes".to_string(), ..Default::default() };
    let mut images = Vec::new();

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    for (_key, entry) in entries {
        let name = match entry["Name"].as_str() {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };

        let effect = entry["Description"].as_str().unwrap_or_default();
        let trigger_condition = entry["Criteria"].as_str().unwrap_or_default();
        let max_rank = entry["MaxRank"].as_i64().unwrap_or(0) as i32;
        let source_str = entry["Rarity"].as_str().unwrap_or_default();
        let equipment_type = entry["Type"].as_str().unwrap_or_default();
        let image = entry["Image"].as_str().map(|s| s.to_string());
        let icon_path = image.as_ref()
            .map(|img| format!("assets/arcanes/{img}"))
            .unwrap_or_default();

        match tx.execute(
            "INSERT INTO arcanes (name, trigger_condition, effect, max_rank, source, equipment_type, icon_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(name) DO UPDATE SET
             trigger_condition=excluded.trigger_condition, effect=excluded.effect,
             max_rank=excluded.max_rank, source=excluded.source,
             equipment_type=excluded.equipment_type, icon_path=excluded.icon_path",
            params![name, trigger_condition, effect, max_rank, source_str, equipment_type, icon_path],
        ) {
            Ok(_) => report.inserted += 1,
            Err(e) => {
                eprintln!("Failed to upsert arcane {name}: {e}");
                report.failed += 1;
            }
        }

        if let Some(img) = image {
            images.push(ImageTask { wiki_filename: img, local_subdir: "arcanes".to_string() });
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
    fn test_process_arcanes_basic() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Arcane Acceleration"] = {
                Name = "Arcane Acceleration",
                Description = "On Critical Hit: 60% chance to increase Fire Rate of Rifles by 45% for 9s.",
                Criteria = "On Critical Hit",
                MaxRank = 5,
                Rarity = "Rare",
                Type = "Warframe",
                Image = "ArcaneAcceleration.png"
            }
        }"#).unwrap();

        let result = process_arcanes_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);
        assert_eq!(result.report.failed, 0);
        assert_eq!(result.images.len(), 1);
        assert_eq!(result.images[0].wiki_filename, "ArcaneAcceleration.png");
        assert_eq!(result.images[0].local_subdir, "arcanes");

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM arcanes", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        let (name, effect, trigger, max_rank, source, equip_type, icon_path): (String, String, String, i32, String, String, String) = conn.query_row(
            "SELECT name, effect, trigger_condition, max_rank, source, equipment_type, icon_path FROM arcanes WHERE name = 'Arcane Acceleration'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        ).unwrap();
        assert_eq!(name, "Arcane Acceleration");
        assert_eq!(effect, "On Critical Hit: 60% chance to increase Fire Rate of Rifles by 45% for 9s.");
        assert_eq!(trigger, "On Critical Hit");
        assert_eq!(max_rank, 5);
        assert_eq!(source, "Rare");
        assert_eq!(equip_type, "Warframe");
        assert_eq!(icon_path, "assets/arcanes/ArcaneAcceleration.png");
    }

    #[test]
    fn test_arcanes_skip_no_name() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["NoName"] = {
                Description = "Missing name field.",
                MaxRank = 3
            },
            ["Arcane Guardian"] = {
                Name = "Arcane Guardian",
                Description = "On Damaged: 60% chance to increase Armor by 600 for 20s.",
                Criteria = "On Damaged",
                MaxRank = 5,
                Rarity = "Rare",
                Type = "Warframe",
                Image = "ArcaneGuardian.png"
            }
        }"#).unwrap();

        let result = process_arcanes_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM arcanes", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_arcanes_upsert_updates_existing() {
        let conn = test_db();

        let data1 = lua_parser::eval_lua_module(r#"return {
            ["Arcane Energize"] = {
                Name = "Arcane Energize",
                Description = "Old description.",
                Criteria = "",
                MaxRank = 5,
                Rarity = "Legendary",
                Type = "Warframe",
                Image = "ArcaneEnergize.png"
            }
        }"#).unwrap();
        process_arcanes_data(&conn, &data1).unwrap();

        let data2 = lua_parser::eval_lua_module(r#"return {
            ["Arcane Energize"] = {
                Name = "Arcane Energize",
                Description = "On Energy Pickup: 40% chance to replenish 150 Energy to nearby allies.",
                Criteria = "On Energy Pickup",
                MaxRank = 5,
                Rarity = "Legendary",
                Type = "Warframe",
                Image = "ArcaneEnergize.png"
            }
        }"#).unwrap();
        process_arcanes_data(&conn, &data2).unwrap();

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM arcanes", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        let effect: String = conn.query_row(
            "SELECT effect FROM arcanes WHERE name = 'Arcane Energize'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(effect, "On Energy Pickup: 40% chance to replenish 150 Energy to nearby allies.");
    }

    #[test]
    fn test_arcanes_no_image() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Arcane Avenger"] = {
                Name = "Arcane Avenger",
                Description = "On Damaged: 21% chance to increase Critical Chance by 45% for 12s.",
                Criteria = "On Damaged",
                MaxRank = 5,
                Rarity = "Rare",
                Type = "Warframe"
            }
        }"#).unwrap();

        let result = process_arcanes_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);
        assert_eq!(result.images.len(), 0);

        let icon_path: String = conn.query_row(
            "SELECT icon_path FROM arcanes WHERE name = 'Arcane Avenger'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(icon_path, "");
    }
}
