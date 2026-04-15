use rusqlite::{params, Connection};
use serde_json::Value;
use crate::fetcher::{CategoryResult, CategoryReport, ImageTask};
use crate::fetcher::wiki_client::WikiClient;
use crate::fetcher::lua_parser;

const PHYSICAL_TYPES: &[&str] = &["Impact", "Puncture", "Slash"];
const PRIMARY_TYPES: &[&str] = &["Heat", "Cold", "Electricity", "Toxin"];

fn classify_element(name: &str) -> &'static str {
    if PHYSICAL_TYPES.contains(&name) {
        "physical"
    } else if PRIMARY_TYPES.contains(&name) {
        "primary"
    } else {
        "combined"
    }
}

pub fn fetch_damage_types(conn: &Connection, wiki: &WikiClient) -> Result<CategoryResult, String> {
    let source = wiki.fetch_module_source("Module:DamageTypes/data")?;
    let data = lua_parser::eval_lua_module(&source)?;
    process_damage_types_data(conn, &data)
}

pub fn process_damage_types_data(conn: &Connection, data: &Value) -> Result<CategoryResult, String> {
    let entries = data.as_object().ok_or("damage types data: expected object")?;
    let mut report = CategoryReport { category: "damage_types".to_string(), ..Default::default() };
    let images: Vec<ImageTask> = Vec::new();

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    // Delete all existing faction_weaknesses before re-inserting
    tx.execute("DELETE FROM faction_weaknesses", []).map_err(|e| e.to_string())?;

    for (_key, entry) in entries {
        let name = match entry["Name"].as_str() {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };

        let element_type = classify_element(name);
        let status_effect = entry["Status"].as_str().unwrap_or_default();

        // For combined elements, extract component_a and component_b
        // These may appear as "Components", "Combine", or similar keys
        let (component_a, component_b) = if element_type == "combined" {
            let comp_a = entry["ComponentA"].as_str()
                .or_else(|| entry["component_a"].as_str())
                .unwrap_or_default()
                .to_string();
            let comp_b = entry["ComponentB"].as_str()
                .or_else(|| entry["component_b"].as_str())
                .unwrap_or_default()
                .to_string();
            (comp_a, comp_b)
        } else {
            (String::new(), String::new())
        };

        match tx.execute(
            "INSERT INTO elements (name, element_type, status_effect, component_a, component_b)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(name) DO UPDATE SET
             element_type=excluded.element_type, status_effect=excluded.status_effect,
             component_a=excluded.component_a, component_b=excluded.component_b",
            params![name, element_type, status_effect, component_a, component_b],
        ) {
            Ok(_) => report.inserted += 1,
            Err(e) => {
                eprintln!("Failed to upsert element {name}: {e}");
                report.failed += 1;
                continue;
            }
        }

        // Insert faction weaknesses from Positives/Negatives or Bonus/Penalty fields
        let positives_val = if !entry["Positives"].is_null() {
            Some(&entry["Positives"])
        } else if !entry["Bonus"].is_null() {
            Some(&entry["Bonus"])
        } else {
            None
        };

        let negatives_val = if !entry["Negatives"].is_null() {
            Some(&entry["Negatives"])
        } else if !entry["Penalty"].is_null() {
            Some(&entry["Penalty"])
        } else {
            None
        };

        if let Some(positives) = positives_val {
            insert_faction_weaknesses(&tx, name, positives, "weak");
        }
        if let Some(negatives) = negatives_val {
            insert_faction_weaknesses(&tx, name, negatives, "strong");
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(CategoryResult { report, images })
}

fn insert_faction_weaknesses(tx: &Connection, element_name: &str, factions_val: &Value, relation: &str) {
    // factions_val may be an array of faction names or an object keyed by faction name
    match factions_val {
        Value::Array(arr) => {
            for faction_val in arr {
                if let Some(faction) = faction_val.as_str() {
                    let (weak_element, strong_element) = if relation == "weak" {
                        (element_name, "")
                    } else {
                        ("", element_name)
                    };
                    if let Err(e) = tx.execute(
                        "INSERT INTO faction_weaknesses (faction, armor_type, weak_element, strong_element)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![faction, "", weak_element, strong_element],
                    ) {
                        eprintln!("Failed to insert faction weakness for {element_name}/{faction}: {e}");
                    }
                }
            }
        }
        Value::Object(map) => {
            for (faction, _) in map {
                let (weak_element, strong_element) = if relation == "weak" {
                    (element_name, "")
                } else {
                    ("", element_name)
                };
                if let Err(e) = tx.execute(
                    "INSERT INTO faction_weaknesses (faction, armor_type, weak_element, strong_element)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![faction, "", weak_element, strong_element],
                ) {
                    eprintln!("Failed to insert faction weakness for {element_name}/{faction}: {e}");
                }
            }
        }
        Value::String(faction) => {
            let (weak_element, strong_element) = if relation == "weak" {
                (element_name, "")
            } else {
                ("", element_name)
            };
            if let Err(e) = tx.execute(
                "INSERT INTO faction_weaknesses (faction, armor_type, weak_element, strong_element)
                 VALUES (?1, ?2, ?3, ?4)",
                params![faction, "", weak_element, strong_element],
            ) {
                eprintln!("Failed to insert faction weakness for {element_name}/{faction}: {e}");
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;
    use crate::fetcher::lua_parser;

    #[test]
    fn test_process_physical_element_impact() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Impact"] = {
                Name = "Impact",
                Status = "Knockback"
            }
        }"#).unwrap();

        let result = process_damage_types_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);
        assert_eq!(result.report.failed, 0);
        assert_eq!(result.images.len(), 0);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM elements", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        let (name, element_type, status_effect, comp_a, comp_b): (String, String, String, String, String) = conn.query_row(
            "SELECT name, element_type, status_effect, component_a, component_b FROM elements WHERE name = 'Impact'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        ).unwrap();
        assert_eq!(name, "Impact");
        assert_eq!(element_type, "physical");
        assert_eq!(status_effect, "Knockback");
        assert_eq!(comp_a, "");
        assert_eq!(comp_b, "");
    }

    #[test]
    fn test_process_combined_element_corrosive() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Corrosive"] = {
                Name = "Corrosive",
                Status = "Corrosion",
                ComponentA = "Electricity",
                ComponentB = "Toxin"
            }
        }"#).unwrap();

        let result = process_damage_types_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);
        assert_eq!(result.report.failed, 0);

        let (element_type, comp_a, comp_b): (String, String, String) = conn.query_row(
            "SELECT element_type, component_a, component_b FROM elements WHERE name = 'Corrosive'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).unwrap();
        assert_eq!(element_type, "combined");
        assert_eq!(comp_a, "Electricity");
        assert_eq!(comp_b, "Toxin");
    }

    #[test]
    fn test_element_type_classification() {
        assert_eq!(classify_element("Impact"), "physical");
        assert_eq!(classify_element("Puncture"), "physical");
        assert_eq!(classify_element("Slash"), "physical");
        assert_eq!(classify_element("Heat"), "primary");
        assert_eq!(classify_element("Cold"), "primary");
        assert_eq!(classify_element("Electricity"), "primary");
        assert_eq!(classify_element("Toxin"), "primary");
        assert_eq!(classify_element("Corrosive"), "combined");
        assert_eq!(classify_element("Blast"), "combined");
        assert_eq!(classify_element("Magnetic"), "combined");
        assert_eq!(classify_element("Radiation"), "combined");
    }

    #[test]
    fn test_faction_weaknesses_inserted_from_positives_array() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Corrosive"] = {
                Name = "Corrosive",
                Status = "Corrosion",
                ComponentA = "Electricity",
                ComponentB = "Toxin",
                Positives = { "Grineer", "Infested" }
            }
        }"#).unwrap();

        process_damage_types_data(&conn, &data).unwrap();

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM faction_weaknesses WHERE weak_element = 'Corrosive'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_faction_weaknesses_deleted_on_reinsert() {
        let conn = test_db();

        let data1 = lua_parser::eval_lua_module(r#"return {
            ["Impact"] = {
                Name = "Impact",
                Status = "Knockback",
                Positives = { "Grineer" }
            }
        }"#).unwrap();
        process_damage_types_data(&conn, &data1).unwrap();

        let count1: i64 = conn.query_row("SELECT COUNT(*) FROM faction_weaknesses", [], |r| r.get(0)).unwrap();
        assert_eq!(count1, 1);

        // Re-run with no faction weaknesses — table should be cleared
        let data2 = lua_parser::eval_lua_module(r#"return {
            ["Impact"] = {
                Name = "Impact",
                Status = "Knockback"
            }
        }"#).unwrap();
        process_damage_types_data(&conn, &data2).unwrap();

        let count2: i64 = conn.query_row("SELECT COUNT(*) FROM faction_weaknesses", [], |r| r.get(0)).unwrap();
        assert_eq!(count2, 0);
    }

    #[test]
    fn test_elements_upsert_updates_existing() {
        let conn = test_db();

        let data1 = lua_parser::eval_lua_module(r#"return {
            ["Heat"] = {
                Name = "Heat",
                Status = "OldStatus"
            }
        }"#).unwrap();
        process_damage_types_data(&conn, &data1).unwrap();

        let data2 = lua_parser::eval_lua_module(r#"return {
            ["Heat"] = {
                Name = "Heat",
                Status = "Incineration"
            }
        }"#).unwrap();
        process_damage_types_data(&conn, &data2).unwrap();

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM elements", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        let status: String = conn.query_row(
            "SELECT status_effect FROM elements WHERE name = 'Heat'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(status, "Incineration");
    }
}
