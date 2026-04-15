use rusqlite::{params, Connection};
use serde_json::Value;
use crate::fetcher::{CategoryResult, CategoryReport, ImageTask};
use crate::fetcher::wiki_client::WikiClient;
use crate::fetcher::lua_parser;

pub fn fetch_relics(conn: &Connection, wiki: &WikiClient) -> Result<CategoryResult, String> {
    let source = wiki.fetch_module_source("Module:Void/data")?;
    let data = lua_parser::eval_lua_module(&source)?;
    process_relics_data(conn, &data)
}

pub fn process_relics_data(conn: &Connection, data: &Value) -> Result<CategoryResult, String> {
    let entries = data.as_object().ok_or("relics data: expected object")?;
    let mut report = CategoryReport { category: "relics".to_string(), ..Default::default() };
    let images: Vec<ImageTask> = Vec::new();

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    for (key, entry) in entries {
        // Name may come from a "Name" field, or fall back to the key
        let name = entry["Name"].as_str()
            .filter(|n| !n.is_empty())
            .unwrap_or(key.as_str());

        if name.is_empty() {
            continue;
        }

        let era = entry["Tier"].as_str().unwrap_or_default();
        let is_vaulted = entry["Vaulted"].as_bool().unwrap_or(false);

        match tx.execute(
            "INSERT INTO relics (name, era, is_vaulted)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(name) DO UPDATE SET
             era=excluded.era, is_vaulted=excluded.is_vaulted",
            params![name, era, is_vaulted],
        ) {
            Ok(_) => report.inserted += 1,
            Err(e) => {
                eprintln!("Failed to upsert relic {name}: {e}");
                report.failed += 1;
                continue;
            }
        }

        let relic_id: i64 = match tx.query_row(
            "SELECT id FROM relics WHERE name = ?1",
            params![name],
            |r| r.get(0),
        ) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("Failed to retrieve relic id for {name}: {e}");
                continue;
            }
        };

        // Delete existing rewards for this relic before re-inserting
        if let Err(e) = tx.execute(
            "DELETE FROM relic_rewards WHERE relic_id = ?1",
            params![relic_id],
        ) {
            eprintln!("Failed to delete rewards for relic {name}: {e}");
        }

        // Rewards may live under a "Drops" key (array or object)
        let drops_val = &entry["Drops"];
        match drops_val {
            Value::Array(arr) => {
                for drop in arr {
                    insert_relic_reward(&tx, relic_id, drop, name);
                }
            }
            Value::Object(map) => {
                for (_drop_key, drop) in map {
                    insert_relic_reward(&tx, relic_id, drop, name);
                }
            }
            _ => {}
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(CategoryResult { report, images })
}

fn insert_relic_reward(tx: &Connection, relic_id: i64, drop: &Value, relic_name: &str) {
    let item = drop["Item"].as_str().unwrap_or_default();
    let part = drop["Part"].as_str().unwrap_or_default();
    let item_name = if part.is_empty() {
        item.to_string()
    } else {
        format!("{item} {part}")
    };

    if item_name.trim().is_empty() {
        return;
    }

    let rarity = drop["Rarity"].as_str().unwrap_or_default();

    if let Err(e) = tx.execute(
        "INSERT INTO relic_rewards (relic_id, item_name, item_type, rarity)
         VALUES (?1, ?2, ?3, ?4)",
        params![relic_id, item_name, "", rarity],
    ) {
        eprintln!("Failed to insert reward {item_name} for relic {relic_name}: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;
    use crate::fetcher::lua_parser;

    #[test]
    fn test_process_relics_basic() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Lith A1"] = {
                Name = "Lith A1",
                Tier = "Lith",
                Vaulted = false,
                Drops = {
                    { Item = "Ash", Part = "Neuroptics Blueprint", Rarity = "Common" },
                    { Item = "Ash", Part = "Systems Blueprint", Rarity = "Uncommon" },
                    { Item = "Ash", Part = "Chassis Blueprint", Rarity = "Rare" }
                }
            }
        }"#).unwrap();

        let result = process_relics_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);
        assert_eq!(result.report.failed, 0);
        assert_eq!(result.images.len(), 0);

        let relic_count: i64 = conn.query_row("SELECT COUNT(*) FROM relics", [], |r| r.get(0)).unwrap();
        assert_eq!(relic_count, 1);

        let reward_count: i64 = conn.query_row("SELECT COUNT(*) FROM relic_rewards", [], |r| r.get(0)).unwrap();
        assert_eq!(reward_count, 3);

        let (name, era, is_vaulted): (String, String, bool) = conn.query_row(
            "SELECT name, era, is_vaulted FROM relics WHERE name = 'Lith A1'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).unwrap();
        assert_eq!(name, "Lith A1");
        assert_eq!(era, "Lith");
        assert!(!is_vaulted);
    }

    #[test]
    fn test_relic_rewards_concatenated_item_name() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Meso N1"] = {
                Name = "Meso N1",
                Tier = "Meso",
                Vaulted = false,
                Drops = {
                    { Item = "Nova", Part = "Systems Blueprint", Rarity = "Rare" }
                }
            }
        }"#).unwrap();

        process_relics_data(&conn, &data).unwrap();

        let item_name: String = conn.query_row(
            "SELECT item_name FROM relic_rewards LIMIT 1",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(item_name, "Nova Systems Blueprint");
    }

    #[test]
    fn test_relic_rewards_deleted_on_upsert() {
        let conn = test_db();

        let data1 = lua_parser::eval_lua_module(r#"return {
            ["Neo A1"] = {
                Name = "Neo A1",
                Tier = "Neo",
                Vaulted = false,
                Drops = {
                    { Item = "Saryn", Part = "Neuroptics Blueprint", Rarity = "Common" },
                    { Item = "Saryn", Part = "Systems Blueprint", Rarity = "Uncommon" }
                }
            }
        }"#).unwrap();
        process_relics_data(&conn, &data1).unwrap();

        let reward_count1: i64 = conn.query_row("SELECT COUNT(*) FROM relic_rewards", [], |r| r.get(0)).unwrap();
        assert_eq!(reward_count1, 2);

        // Re-insert with only 1 reward — old rewards must be deleted
        let data2 = lua_parser::eval_lua_module(r#"return {
            ["Neo A1"] = {
                Name = "Neo A1",
                Tier = "Neo",
                Vaulted = true,
                Drops = {
                    { Item = "Saryn", Part = "Neuroptics Blueprint", Rarity = "Common" }
                }
            }
        }"#).unwrap();
        process_relics_data(&conn, &data2).unwrap();

        let relic_count: i64 = conn.query_row("SELECT COUNT(*) FROM relics", [], |r| r.get(0)).unwrap();
        assert_eq!(relic_count, 1);

        let reward_count2: i64 = conn.query_row("SELECT COUNT(*) FROM relic_rewards", [], |r| r.get(0)).unwrap();
        assert_eq!(reward_count2, 1);

        let is_vaulted: bool = conn.query_row(
            "SELECT is_vaulted FROM relics WHERE name = 'Neo A1'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert!(is_vaulted);
    }

    #[test]
    fn test_relic_reward_rarity() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Axi A1"] = {
                Name = "Axi A1",
                Tier = "Axi",
                Vaulted = false,
                Drops = {
                    { Item = "Volt", Part = "Neuroptics Blueprint", Rarity = "Common" },
                    { Item = "Volt", Part = "Systems Blueprint", Rarity = "Uncommon" },
                    { Item = "Volt", Part = "Chassis Blueprint", Rarity = "Rare" }
                }
            }
        }"#).unwrap();

        process_relics_data(&conn, &data).unwrap();

        let rarities: Vec<String> = {
            let mut stmt = conn.prepare(
                "SELECT rarity FROM relic_rewards ORDER BY rarity"
            ).unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .map(|r| r.unwrap())
                .collect()
        };
        assert!(rarities.contains(&"Common".to_string()));
        assert!(rarities.contains(&"Uncommon".to_string()));
        assert!(rarities.contains(&"Rare".to_string()));
    }

    #[test]
    fn test_relic_key_fallback_for_name() {
        // If no Name field, the Lua key itself should be used as relic name
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Lith B3"] = {
                Tier = "Lith",
                Vaulted = true,
                Drops = {}
            }
        }"#).unwrap();

        let result = process_relics_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);

        let name: String = conn.query_row(
            "SELECT name FROM relics WHERE name = 'Lith B3'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(name, "Lith B3");
    }
}
