use rusqlite::{params, Connection};
use serde_json::Value;
use crate::fetcher::{CategoryResult, CategoryReport, ImageTask};
use crate::fetcher::wiki_client::WikiClient;
use crate::fetcher::lua_parser;

pub fn fetch_planets(conn: &Connection, wiki: &WikiClient) -> Result<CategoryResult, String> {
    let source = wiki.fetch_module_source("Module:Missions/data")?;
    let data = lua_parser::eval_lua_module(&source)?;
    process_planets_data(conn, &data)
}

pub fn process_planets_data(conn: &Connection, data: &Value) -> Result<CategoryResult, String> {
    let mut report = CategoryReport { category: "planets".to_string(), ..Default::default() };
    let images: Vec<ImageTask> = Vec::new();

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    // The Lua data may have a top-level "Regions" key, or may itself be the region map.
    // Try "Regions" first; fall back to iterating the root object.
    let regions_value: &Value = if data["Regions"].is_object() {
        &data["Regions"]
    } else {
        data
    };

    let regions = match regions_value.as_object() {
        Some(r) => r,
        None => return Ok(CategoryResult { report, images }),
    };

    for (planet_name, region_entry) in regions {
        // Skip non-object entries and metadata keys
        let entry = match region_entry.as_object() {
            Some(e) => e,
            None => continue,
        };

        let faction = region_entry["Faction"].as_str().unwrap_or_default();
        let tileset = region_entry["Tileset"].as_str().unwrap_or_default();
        let open_world = region_entry["OpenWorld"].as_str().unwrap_or_default();
        let hub = region_entry["Hub"].as_str().unwrap_or_default();
        let icon = region_entry["Image"].as_str().map(|s| s.to_string());
        let icon_path = icon.as_ref().map(|img| format!("assets/planets/{img}")).unwrap_or_default();

        match tx.execute(
            "INSERT INTO planets (name, faction, tileset, open_world_name, hub_name, icon_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(name) DO UPDATE SET
             faction=excluded.faction, tileset=excluded.tileset,
             open_world_name=excluded.open_world_name, hub_name=excluded.hub_name,
             icon_path=excluded.icon_path",
            params![planet_name, faction, tileset, open_world, hub, icon_path],
        ) {
            Ok(_) => report.inserted += 1,
            Err(e) => {
                eprintln!("Failed to upsert planet {planet_name}: {e}");
                report.failed += 1;
                continue;
            }
        }

        // Insert resources for this planet
        let planet_id: i64 = match tx.query_row(
            "SELECT id FROM planets WHERE name = ?1",
            params![planet_name],
            |r| r.get(0),
        ) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("Failed to retrieve planet id for {planet_name}: {e}");
                continue;
            }
        };

        // Delete existing resources before re-inserting
        if let Err(e) = tx.execute(
            "DELETE FROM planet_resources WHERE planet_id = ?1",
            params![planet_id],
        ) {
            eprintln!("Failed to delete resources for {planet_name}: {e}");
        }

        // Resources may be a list or a map keyed by resource name
        if let Some(resources) = entry.get("Resources") {
            match resources {
                Value::Array(arr) => {
                    for item in arr {
                        let resource_name = item.as_str().unwrap_or_default();
                        if resource_name.is_empty() {
                            continue;
                        }
                        if let Err(e) = tx.execute(
                            "INSERT INTO planet_resources (planet_id, resource_name, rarity)
                             VALUES (?1, ?2, ?3)",
                            params![planet_id, resource_name, ""],
                        ) {
                            eprintln!("Failed to insert resource {resource_name}: {e}");
                        }
                    }
                }
                Value::Object(map) => {
                    for (resource_name, resource_entry) in map {
                        let rarity = resource_entry["Rarity"].as_str().unwrap_or_default();
                        if let Err(e) = tx.execute(
                            "INSERT INTO planet_resources (planet_id, resource_name, rarity)
                             VALUES (?1, ?2, ?3)",
                            params![planet_id, resource_name, rarity],
                        ) {
                            eprintln!("Failed to insert resource {resource_name}: {e}");
                        }
                    }
                }
                _ => {}
            }
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
    fn test_process_planets_basic() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            Regions = {
                ["Earth"] = {
                    Faction = "Grineer",
                    Tileset = "Earth Forest",
                    Resources = { "Rubedo", "Alloy Plate" }
                }
            }
        }"#).unwrap();

        let result = process_planets_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);
        assert_eq!(result.report.failed, 0);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM planets", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        let (faction, tileset): (String, String) = conn.query_row(
            "SELECT faction, tileset FROM planets WHERE name = 'Earth'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert_eq!(faction, "Grineer");
        assert_eq!(tileset, "Earth Forest");

        let res_count: i64 = conn.query_row("SELECT COUNT(*) FROM planet_resources", [], |r| r.get(0)).unwrap();
        assert_eq!(res_count, 2);
    }

    #[test]
    fn test_process_planets_root_fallback() {
        // When data has no "Regions" key, iterate the root object
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Mars"] = {
                Faction = "Grineer",
                Tileset = "Grineer Settlement"
            }
        }"#).unwrap();

        let result = process_planets_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);

        let faction: String = conn.query_row(
            "SELECT faction FROM planets WHERE name = 'Mars'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(faction, "Grineer");
    }

    #[test]
    fn test_planet_resources_deleted_on_upsert() {
        let conn = test_db();

        // Insert planet with 2 resources
        let data1 = lua_parser::eval_lua_module(r#"return {
            Regions = {
                ["Venus"] = {
                    Faction = "Corpus",
                    Tileset = "Corpus Ship",
                    Resources = { "Polymer Bundle", "Circuits" }
                }
            }
        }"#).unwrap();
        process_planets_data(&conn, &data1).unwrap();

        let res_count: i64 = conn.query_row("SELECT COUNT(*) FROM planet_resources", [], |r| r.get(0)).unwrap();
        assert_eq!(res_count, 2);

        // Re-insert with 1 resource — old ones must be deleted first
        let data2 = lua_parser::eval_lua_module(r#"return {
            Regions = {
                ["Venus"] = {
                    Faction = "Corpus",
                    Tileset = "Corpus Ship",
                    Resources = { "Polymer Bundle" }
                }
            }
        }"#).unwrap();
        process_planets_data(&conn, &data2).unwrap();

        let planet_count: i64 = conn.query_row("SELECT COUNT(*) FROM planets", [], |r| r.get(0)).unwrap();
        assert_eq!(planet_count, 1);

        let res_count2: i64 = conn.query_row("SELECT COUNT(*) FROM planet_resources", [], |r| r.get(0)).unwrap();
        assert_eq!(res_count2, 1);
    }

    #[test]
    fn test_planet_resources_map_form() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            Regions = {
                ["Ceres"] = {
                    Faction = "Grineer",
                    Tileset = "Grineer Galleon",
                    Resources = {
                        ["Ferrite"] = { Rarity = "Common" },
                        ["Nano Spores"] = { Rarity = "Uncommon" }
                    }
                }
            }
        }"#).unwrap();

        process_planets_data(&conn, &data).unwrap();

        let res_count: i64 = conn.query_row("SELECT COUNT(*) FROM planet_resources", [], |r| r.get(0)).unwrap();
        assert_eq!(res_count, 2);
    }

    #[test]
    fn test_planet_upsert_updates_existing() {
        let conn = test_db();

        let data1 = lua_parser::eval_lua_module(r#"return {
            Regions = {
                ["Jupiter"] = { Faction = "Corpus", Tileset = "Corpus Gas City" }
            }
        }"#).unwrap();
        process_planets_data(&conn, &data1).unwrap();

        let data2 = lua_parser::eval_lua_module(r#"return {
            Regions = {
                ["Jupiter"] = { Faction = "Infested", Tileset = "Infested Ship" }
            }
        }"#).unwrap();
        process_planets_data(&conn, &data2).unwrap();

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM planets", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        let faction: String = conn.query_row(
            "SELECT faction FROM planets WHERE name = 'Jupiter'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(faction, "Infested");
    }
}
