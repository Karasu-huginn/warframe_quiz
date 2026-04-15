use rusqlite::{params, Connection};
use serde_json::Value;
use crate::fetcher::{CategoryResult, CategoryReport, ImageTask};
use crate::fetcher::wiki_client::WikiClient;
use crate::fetcher::lua_parser;

pub fn fetch_focus(conn: &Connection, wiki: &WikiClient) -> Result<CategoryResult, String> {
    let source = wiki.fetch_module_source("Module:Focus/data")?;
    let data = lua_parser::eval_lua_module(&source)?;
    process_focus_data(conn, &data)
}

pub fn process_focus_data(conn: &Connection, data: &Value) -> Result<CategoryResult, String> {
    let entries = data.as_object().ok_or("focus data: expected object")?;
    let mut report = CategoryReport { category: "focus".to_string(), ..Default::default() };
    let images: Vec<ImageTask> = Vec::new();

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    for (_key, school_entry) in entries {
        // Each top-level entry should be a school with a Name field (or use the key directly)
        let school_name = match school_entry["Name"].as_str() {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => {
                // If no Name field, the key itself may be the school name — but we
                // can't easily get it here. Skip entries without a Name.
                continue;
            }
        };

        let description = school_entry["Description"].as_str().unwrap_or_default();
        let image = school_entry["Image"].as_str().unwrap_or_default();
        let symbol_path = if image.is_empty() {
            String::new()
        } else {
            image.to_string()
        };

        match tx.execute(
            "INSERT INTO focus_schools (name, description, symbol_path)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(name) DO UPDATE SET
             description=excluded.description, symbol_path=excluded.symbol_path",
            params![school_name, description, symbol_path],
        ) {
            Ok(_) => report.inserted += 1,
            Err(e) => {
                eprintln!("Failed to upsert focus school {school_name}: {e}");
                report.failed += 1;
                continue;
            }
        }

        let school_id: i64 = match tx.query_row(
            "SELECT id FROM focus_schools WHERE name = ?1",
            params![school_name],
            |r| r.get(0),
        ) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("Failed to retrieve school id for {school_name}: {e}");
                continue;
            }
        };

        // Delete existing abilities for this school before re-inserting
        if let Err(e) = tx.execute(
            "DELETE FROM focus_abilities WHERE school_id = ?1",
            params![school_id],
        ) {
            eprintln!("Failed to delete abilities for school {school_name}: {e}");
        }

        // Abilities may live under an "Abilities" key (array or object)
        // or under a "Powers" key, depending on the wiki module structure.
        let abilities_value = if !school_entry["Abilities"].is_null() {
            &school_entry["Abilities"]
        } else if !school_entry["Powers"].is_null() {
            &school_entry["Powers"]
        } else {
            continue;
        };

        match abilities_value {
            Value::Array(arr) => {
                for ability in arr {
                    insert_focus_ability(&tx, ability, school_id, &school_name);
                }
            }
            Value::Object(map) => {
                for (_ability_key, ability) in map {
                    insert_focus_ability(&tx, ability, school_id, &school_name);
                }
            }
            _ => {}
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(CategoryResult { report, images })
}

fn insert_focus_ability(
    tx: &Connection,
    ability: &Value,
    school_id: i64,
    school_name: &str,
) {
    let ability_name = match ability["Name"].as_str() {
        Some(n) if !n.is_empty() => n,
        _ => return,
    };

    let description = ability["Description"].as_str().unwrap_or_default();
    let is_waybound = ability["IsWayBound"].as_bool().unwrap_or(false);
    let is_passive = ability["IsPassive"].as_bool().unwrap_or(false);

    if let Err(e) = tx.execute(
        "INSERT INTO focus_abilities (name, description, school_id, is_waybound, is_passive)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![ability_name, description, school_id, is_waybound, is_passive],
    ) {
        eprintln!("Failed to insert focus ability {ability_name} for school {school_name}: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;
    use crate::fetcher::lua_parser;

    #[test]
    fn test_process_focus_basic() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Naramon"] = {
                Name = "Naramon",
                Description = "The Shadow path focuses on cunning and tactics.",
                Image = "Naramon.png",
                Abilities = {
                    {
                        Name = "Power Spike",
                        Description = "Melee kills restore combo count.",
                        IsWayBound = false,
                        IsPassive = true
                    },
                    {
                        Name = "Mind Step",
                        Description = "Increases sprint speed.",
                        IsWayBound = true,
                        IsPassive = false
                    }
                }
            }
        }"#).unwrap();

        let result = process_focus_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);
        assert_eq!(result.report.failed, 0);

        let school_count: i64 = conn.query_row("SELECT COUNT(*) FROM focus_schools", [], |r| r.get(0)).unwrap();
        assert_eq!(school_count, 1);

        let ability_count: i64 = conn.query_row("SELECT COUNT(*) FROM focus_abilities", [], |r| r.get(0)).unwrap();
        assert_eq!(ability_count, 2);

        let (name, description): (String, String) = conn.query_row(
            "SELECT name, description FROM focus_schools WHERE name = 'Naramon'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert_eq!(name, "Naramon");
        assert_eq!(description, "The Shadow path focuses on cunning and tactics.");
    }

    #[test]
    fn test_focus_abilities_waybound_and_passive_flags() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Zenurik"] = {
                Name = "Zenurik",
                Description = "The Energy path.",
                Image = "Zenurik.png",
                Abilities = {
                    {
                        Name = "Energy Pulse",
                        Description = "Enhances energy regeneration.",
                        IsWayBound = true,
                        IsPassive = false
                    },
                    {
                        Name = "Inner Might",
                        Description = "Passive energy bonus.",
                        IsWayBound = false,
                        IsPassive = true
                    }
                }
            }
        }"#).unwrap();

        process_focus_data(&conn, &data).unwrap();

        let (waybound, passive): (bool, bool) = conn.query_row(
            "SELECT is_waybound, is_passive FROM focus_abilities WHERE name = 'Energy Pulse'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert!(waybound);
        assert!(!passive);

        let (waybound2, passive2): (bool, bool) = conn.query_row(
            "SELECT is_waybound, is_passive FROM focus_abilities WHERE name = 'Inner Might'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert!(!waybound2);
        assert!(passive2);
    }

    #[test]
    fn test_focus_abilities_deleted_on_upsert() {
        let conn = test_db();

        // Insert school with 2 abilities
        let data1 = lua_parser::eval_lua_module(r#"return {
            ["Madurai"] = {
                Name = "Madurai",
                Description = "The Power path.",
                Image = "Madurai.png",
                Abilities = {
                    { Name = "Void Strike", Description = "Increases damage.", IsWayBound = false, IsPassive = false },
                    { Name = "Phoenix Talons", Description = "Increases crit chance.", IsWayBound = false, IsPassive = false }
                }
            }
        }"#).unwrap();
        process_focus_data(&conn, &data1).unwrap();

        let ability_count: i64 = conn.query_row("SELECT COUNT(*) FROM focus_abilities", [], |r| r.get(0)).unwrap();
        assert_eq!(ability_count, 2);

        // Re-insert with only 1 ability — old ones must be deleted
        let data2 = lua_parser::eval_lua_module(r#"return {
            ["Madurai"] = {
                Name = "Madurai",
                Description = "The Power path.",
                Image = "Madurai.png",
                Abilities = {
                    { Name = "Void Strike", Description = "Updated description.", IsWayBound = true, IsPassive = false }
                }
            }
        }"#).unwrap();
        process_focus_data(&conn, &data2).unwrap();

        let school_count: i64 = conn.query_row("SELECT COUNT(*) FROM focus_schools", [], |r| r.get(0)).unwrap();
        assert_eq!(school_count, 1);

        let ability_count2: i64 = conn.query_row("SELECT COUNT(*) FROM focus_abilities", [], |r| r.get(0)).unwrap();
        assert_eq!(ability_count2, 1);
    }

    #[test]
    fn test_focus_abilities_object_form() {
        // Test when Abilities is an object (keyed map) rather than an array
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Vazarin"] = {
                Name = "Vazarin",
                Description = "The Ward path.",
                Image = "Vazarin.png",
                Abilities = {
                    ["Protective Dash"] = {
                        Name = "Protective Dash",
                        Description = "Heal allies.",
                        IsWayBound = false,
                        IsPassive = false
                    },
                    ["Mending Soul"] = {
                        Name = "Mending Soul",
                        Description = "Revive downed allies faster.",
                        IsWayBound = true,
                        IsPassive = false
                    }
                }
            }
        }"#).unwrap();

        process_focus_data(&conn, &data).unwrap();

        let ability_count: i64 = conn.query_row("SELECT COUNT(*) FROM focus_abilities", [], |r| r.get(0)).unwrap();
        assert_eq!(ability_count, 2);
    }

    #[test]
    fn test_focus_school_upsert_updates_existing() {
        let conn = test_db();

        let data1 = lua_parser::eval_lua_module(r#"return {
            ["Unairu"] = {
                Name = "Unairu",
                Description = "Old description.",
                Image = "Unairu.png",
                Abilities = {
                    { Name = "Stone Skin", Description = "Reduces damage.", IsWayBound = false, IsPassive = false }
                }
            }
        }"#).unwrap();
        process_focus_data(&conn, &data1).unwrap();

        let data2 = lua_parser::eval_lua_module(r#"return {
            ["Unairu"] = {
                Name = "Unairu",
                Description = "The Stone path.",
                Image = "Unairu2.png",
                Abilities = {
                    { Name = "Stone Skin", Description = "Reduces damage taken.", IsWayBound = false, IsPassive = false }
                }
            }
        }"#).unwrap();
        process_focus_data(&conn, &data2).unwrap();

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM focus_schools", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        let description: String = conn.query_row(
            "SELECT description FROM focus_schools WHERE name = 'Unairu'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(description, "The Stone path.");
    }

    #[test]
    fn test_focus_school_id_fk_in_abilities() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Naramon"] = {
                Name = "Naramon",
                Description = "Shadow path.",
                Image = "Naramon.png",
                Abilities = {
                    { Name = "Power Spike", Description = "Combo retention.", IsWayBound = false, IsPassive = true }
                }
            }
        }"#).unwrap();

        process_focus_data(&conn, &data).unwrap();

        let school_id: i64 = conn.query_row(
            "SELECT id FROM focus_schools WHERE name = 'Naramon'",
            [],
            |r| r.get(0),
        ).unwrap();
        let ability_school_id: i64 = conn.query_row(
            "SELECT school_id FROM focus_abilities WHERE name = 'Power Spike'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(school_id, ability_school_id);
    }
}
