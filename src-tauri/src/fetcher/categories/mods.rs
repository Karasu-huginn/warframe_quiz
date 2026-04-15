use rusqlite::{params, Connection};
use serde_json::Value;
use crate::fetcher::{CategoryResult, CategoryReport, ImageTask};
use crate::fetcher::wiki_client::WikiClient;
use crate::fetcher::lua_parser;

pub fn fetch_mods(conn: &Connection, wiki: &WikiClient) -> Result<CategoryResult, String> {
    let source = wiki.fetch_module_source("Module:Mods/data")?;
    let data = lua_parser::eval_lua_module(&source)?;
    process_mods_data(conn, &data)
}

pub fn process_mods_data(conn: &Connection, data: &Value) -> Result<CategoryResult, String> {
    let entries = data.as_object().ok_or("mods data: expected object")?;
    let mut report = CategoryReport { category: "mods".to_string(), ..Default::default() };
    let mut images = Vec::new();

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    for (_key, entry) in entries {
        let name = match entry["Name"].as_str() {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };

        let polarity = entry["Polarity"].as_str().unwrap_or_default();
        let rarity = entry["Rarity"].as_str().unwrap_or_default();
        let mod_type = entry["Type"].as_str().unwrap_or_default();
        let max_rank = entry["MaxRank"].as_i64().unwrap_or(0) as i32;
        let base_drain = entry["BaseDrain"].as_i64().unwrap_or(0) as i32;
        let effect_description = entry["Description"].as_str().unwrap_or_default();
        let is_exilus = entry["IsExilus"].as_bool().unwrap_or(false);
        let is_augment = entry["IsAbilityAugment"].as_bool().unwrap_or(false);
        let image = entry["Image"].as_str().map(|s| s.to_string());
        let icon_path = image.as_ref().map(|img| format!("assets/mods/{img}")).unwrap_or_default();

        match tx.execute(
            "INSERT INTO mods (name, polarity, rarity, mod_type, max_rank, base_drain,
             effect_description, is_exilus, is_augment, icon_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(name) DO UPDATE SET
             polarity=excluded.polarity, rarity=excluded.rarity, mod_type=excluded.mod_type,
             max_rank=excluded.max_rank, base_drain=excluded.base_drain,
             effect_description=excluded.effect_description, is_exilus=excluded.is_exilus,
             is_augment=excluded.is_augment, icon_path=excluded.icon_path",
            params![
                name, polarity, rarity, mod_type, max_rank, base_drain,
                effect_description, is_exilus, is_augment, icon_path
            ],
        ) {
            Ok(_) => report.inserted += 1,
            Err(e) => {
                eprintln!("Failed to insert mod {name}: {e}");
                report.failed += 1;
            }
        }

        if let Some(img) = image {
            images.push(ImageTask { wiki_filename: img, local_subdir: "mods".to_string() });
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
    fn test_process_mods_serration() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Serration"] = {
                Name = "Serration",
                Polarity = "Madurai",
                Rarity = "Common",
                Type = "Rifle",
                MaxRank = 10,
                BaseDrain = 4,
                Description = "Increases the base damage of rifles.",
                IsExilus = false,
                IsAbilityAugment = false,
                Image = "Serration.png"
            }
        }"#).unwrap();

        let result = process_mods_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);
        assert_eq!(result.report.failed, 0);
        assert_eq!(result.images.len(), 1);
        assert_eq!(result.images[0].wiki_filename, "Serration.png");
        assert_eq!(result.images[0].local_subdir, "mods");

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM mods", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        let (name, mod_type, max_rank, polarity): (String, String, i32, String) = conn.query_row(
            "SELECT name, mod_type, max_rank, polarity FROM mods WHERE name = 'Serration'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        ).unwrap();
        assert_eq!(name, "Serration");
        assert_eq!(mod_type, "Rifle");
        assert_eq!(max_rank, 10);
        assert_eq!(polarity, "Madurai");
    }

    #[test]
    fn test_mods_skip_no_name() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["NoName"] = {
                Polarity = "Madurai",
                Rarity = "Common"
            },
            ["Redirection"] = {
                Name = "Redirection",
                Polarity = "Vazarin",
                Rarity = "Common",
                Type = "Warframe",
                MaxRank = 10,
                BaseDrain = 4,
                Description = "Increases maximum shield capacity.",
                IsExilus = false,
                IsAbilityAugment = false,
                Image = "Redirection.png"
            }
        }"#).unwrap();

        let result = process_mods_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM mods", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_mods_exilus_and_augment_flags() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Vigilante Pursuit"] = {
                Name = "Vigilante Pursuit",
                Polarity = "Naramon",
                Rarity = "Uncommon",
                Type = "Rifle",
                MaxRank = 3,
                BaseDrain = 4,
                Description = "Reveals the location of enemies.",
                IsExilus = true,
                IsAbilityAugment = false,
                Image = "VigPursuit.png"
            }
        }"#).unwrap();

        process_mods_data(&conn, &data).unwrap();

        let is_exilus: bool = conn.query_row(
            "SELECT is_exilus FROM mods WHERE name = 'Vigilante Pursuit'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert!(is_exilus);
    }

    #[test]
    fn test_mods_icon_path_prefix() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Intensify"] = {
                Name = "Intensify",
                Polarity = "Madurai",
                Rarity = "Common",
                Type = "Warframe",
                MaxRank = 5,
                BaseDrain = 4,
                Description = "Increases Ability Strength.",
                IsExilus = false,
                IsAbilityAugment = false,
                Image = "Intensify.png"
            }
        }"#).unwrap();

        process_mods_data(&conn, &data).unwrap();

        let icon_path: String = conn.query_row(
            "SELECT icon_path FROM mods WHERE name = 'Intensify'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(icon_path, "assets/mods/Intensify.png");
    }
}
