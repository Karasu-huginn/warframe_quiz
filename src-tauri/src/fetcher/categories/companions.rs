use rusqlite::{params, Connection};
use serde_json::Value;
use crate::fetcher::{CategoryResult, CategoryReport, ImageTask};
use crate::fetcher::wiki_client::WikiClient;
use crate::fetcher::lua_parser;

pub fn fetch_companions(conn: &Connection, wiki: &WikiClient) -> Result<CategoryResult, String> {
    let source = wiki.fetch_module_source("Module:Companions/data")?;
    let data = lua_parser::eval_lua_module(&source)?;
    process_companions_data(conn, &data)
}

pub fn process_companions_data(conn: &Connection, data: &Value) -> Result<CategoryResult, String> {
    let entries = data.as_object().ok_or("companions data: expected object")?;
    let mut report = CategoryReport { category: "companions".to_string(), ..Default::default() };
    let mut images = Vec::new();

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    for (_key, entry) in entries {
        let name = match entry["Name"].as_str() {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };

        let class = entry["Type"].as_str().unwrap_or_default();
        let health = entry["Health"].as_f64();
        let armor = entry["Armor"].as_f64();
        let shields = entry["Shield"].as_f64();
        let description = entry["Description"].as_str().unwrap_or_default();
        let image = entry["Image"].as_str().map(|s| s.to_string());
        let icon_path = image.as_ref().map(|img| format!("assets/companions/{img}")).unwrap_or_default();

        let companion_id: i64 = match tx.query_row(
            "INSERT INTO companions (name, class, health, shields, armor, description, icon_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             RETURNING id",
            params![name, class, health, shields, armor, description, icon_path],
            |r| r.get(0),
        ) {
            Ok(id) => {
                report.inserted += 1;
                id
            }
            Err(e) => {
                eprintln!("Failed to insert companion {name}: {e}");
                report.failed += 1;
                continue;
            }
        };

        // Insert precepts if present (check common field names for ability/precept data)
        insert_precepts(&tx, companion_id, entry);

        if let Some(img) = image {
            images.push(ImageTask { wiki_filename: img, local_subdir: "companions".to_string() });
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(CategoryResult { report, images })
}

/// Insert precept entries for a companion if precept data is present in the Lua entry.
/// The Lua data may store precepts as an array under "Precepts", "Abilities", or similar keys.
fn insert_precepts(tx: &Connection, companion_id: i64, entry: &Value) {
    // Try "Precepts" first, then "Abilities"
    let precepts_val = if !entry["Precepts"].is_null() {
        &entry["Precepts"]
    } else if !entry["Abilities"].is_null() {
        &entry["Abilities"]
    } else {
        return;
    };

    // Delete existing precepts for this companion before re-inserting
    if let Err(e) = tx.execute(
        "DELETE FROM companion_precepts WHERE companion_id = ?1",
        params![companion_id],
    ) {
        eprintln!("Failed to delete old precepts for companion_id {companion_id}: {e}");
        return;
    }

    // Precepts may be an array of strings or objects
    let precept_list = match precepts_val.as_array() {
        Some(arr) => arr,
        None => return,
    };

    for precept in precept_list {
        let (name, description) = if let Some(s) = precept.as_str() {
            (s.to_string(), String::new())
        } else if let Some(obj) = precept.as_object() {
            let n = obj.get("Name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let d = obj.get("Description")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            (n, d)
        } else {
            continue;
        };

        if name.is_empty() {
            continue;
        }

        if let Err(e) = tx.execute(
            "INSERT INTO companion_precepts (name, description, companion_id) VALUES (?1, ?2, ?3)",
            params![name, description, companion_id],
        ) {
            eprintln!("Failed to insert precept '{name}' for companion_id {companion_id}: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;
    use crate::fetcher::lua_parser;

    #[test]
    fn test_process_companions_basic() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Carrier"] = {
                Name = "Carrier",
                Type = "Sentinel",
                Health = 100,
                Shield = 150,
                Armor = 50,
                Description = "A helpful sentinel.",
                Image = "Carrier.png"
            }
        }"#).unwrap();

        let result = process_companions_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);
        assert_eq!(result.report.failed, 0);
        assert_eq!(result.images.len(), 1);
        assert_eq!(result.images[0].wiki_filename, "Carrier.png");
        assert_eq!(result.images[0].local_subdir, "companions");

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM companions", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        let (name, class): (String, String) = conn.query_row(
            "SELECT name, class FROM companions WHERE name = 'Carrier'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert_eq!(name, "Carrier");
        assert_eq!(class, "Sentinel");
    }

    #[test]
    fn test_companions_icon_path_prefix() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Kubrow"] = {
                Name = "Helminth Charger",
                Type = "Kubrow",
                Health = 200,
                Shield = 0,
                Armor = 100,
                Description = "An infested beast.",
                Image = "HelminthCharger.png"
            }
        }"#).unwrap();

        process_companions_data(&conn, &data).unwrap();

        let icon_path: String = conn.query_row(
            "SELECT icon_path FROM companions WHERE name = 'Helminth Charger'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(icon_path, "assets/companions/HelminthCharger.png");
    }

    #[test]
    fn test_companions_skip_no_name() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["NoName"] = {
                Type = "Sentinel",
                Health = 100
            },
            ["Oxylus"] = {
                Name = "Oxylus",
                Type = "Sentinel",
                Health = 100,
                Shield = 50,
                Armor = 25,
                Description = "Conservation helper.",
                Image = "Oxylus.png"
            }
        }"#).unwrap();

        let result = process_companions_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM companions", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_companions_with_precepts() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Shade"] = {
                Name = "Shade",
                Type = "Sentinel",
                Health = 100,
                Shield = 100,
                Armor = 50,
                Description = "Cloaks its owner.",
                Image = "Shade.png",
                Precepts = { "Ghost", "Revenge" }
            }
        }"#).unwrap();

        process_companions_data(&conn, &data).unwrap();

        let companion_id: i64 = conn.query_row(
            "SELECT id FROM companions WHERE name = 'Shade'",
            [],
            |r| r.get(0),
        ).unwrap();

        let precept_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM companion_precepts WHERE companion_id = ?1",
            params![companion_id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(precept_count, 2);
    }

    #[test]
    fn test_companions_no_precepts_skipped_gracefully() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["MOA"] = {
                Name = "MOA",
                Type = "MOA",
                Health = 300,
                Shield = 100,
                Armor = 0,
                Description = "A robotic companion.",
                Image = "MOA.png"
            }
        }"#).unwrap();

        let result = process_companions_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);

        let precept_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM companion_precepts",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(precept_count, 0);
    }
}
