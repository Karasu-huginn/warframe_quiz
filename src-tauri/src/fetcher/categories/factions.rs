use rusqlite::{params, Connection};
use serde_json::Value;
use crate::fetcher::{CategoryResult, CategoryReport, ImageTask};
use crate::fetcher::wiki_client::WikiClient;
use crate::fetcher::lua_parser;

pub fn fetch_factions(conn: &Connection, wiki: &WikiClient) -> Result<CategoryResult, String> {
    let source = wiki.fetch_module_source("Module:Factions/data")?;
    let data = lua_parser::eval_lua_module(&source)?;
    process_factions_data(conn, &data)
}

pub fn process_factions_data(conn: &Connection, data: &Value) -> Result<CategoryResult, String> {
    let entries = data.as_object().ok_or("factions data: expected object")?;
    let mut report = CategoryReport { category: "factions".to_string(), ..Default::default() };
    let mut images = Vec::new();

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    for (_key, entry) in entries {
        let name = match entry["Name"].as_str() {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };

        let description = entry["Description"].as_str().unwrap_or_default();
        let image = entry["Image"].as_str().map(|s| s.to_string());
        let sigil_path = image.as_ref()
            .map(|img| format!("assets/factions/{img}"))
            .unwrap_or_default();

        match tx.execute(
            "INSERT INTO syndicates (name, description, sigil_path)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(name) DO UPDATE SET
             description=excluded.description, sigil_path=excluded.sigil_path",
            params![name, description, sigil_path],
        ) {
            Ok(_) => report.inserted += 1,
            Err(e) => {
                eprintln!("Failed to upsert faction {name}: {e}");
                report.failed += 1;
            }
        }

        if let Some(img) = image {
            images.push(ImageTask { wiki_filename: img, local_subdir: "factions".to_string() });
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
    fn test_process_factions_basic() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Grineer"] = {
                Name = "Grineer",
                Description = "A militaristic faction of cloned soldiers.",
                Image = "Grineer.png"
            }
        }"#).unwrap();

        let result = process_factions_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);
        assert_eq!(result.report.failed, 0);
        assert_eq!(result.images.len(), 1);
        assert_eq!(result.images[0].wiki_filename, "Grineer.png");
        assert_eq!(result.images[0].local_subdir, "factions");

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM syndicates", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        let (name, description, sigil_path): (String, String, String) = conn.query_row(
            "SELECT name, description, sigil_path FROM syndicates WHERE name = 'Grineer'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).unwrap();
        assert_eq!(name, "Grineer");
        assert_eq!(description, "A militaristic faction of cloned soldiers.");
        assert_eq!(sigil_path, "assets/factions/Grineer.png");
    }

    #[test]
    fn test_factions_skip_no_name() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["NoName"] = {
                Description = "Missing name field."
            },
            ["Corpus"] = {
                Name = "Corpus",
                Description = "A profit-driven merchant faction.",
                Image = "Corpus.png"
            }
        }"#).unwrap();

        let result = process_factions_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM syndicates", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_factions_upsert_updates_existing() {
        let conn = test_db();

        let data1 = lua_parser::eval_lua_module(r#"return {
            ["Infested"] = {
                Name = "Infested",
                Description = "Old description.",
                Image = "Infested.png"
            }
        }"#).unwrap();
        process_factions_data(&conn, &data1).unwrap();

        let data2 = lua_parser::eval_lua_module(r#"return {
            ["Infested"] = {
                Name = "Infested",
                Description = "Corrupted biomass creatures.",
                Image = "Infested2.png"
            }
        }"#).unwrap();
        process_factions_data(&conn, &data2).unwrap();

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM syndicates", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        let description: String = conn.query_row(
            "SELECT description FROM syndicates WHERE name = 'Infested'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(description, "Corrupted biomass creatures.");
    }

    #[test]
    fn test_factions_sigil_path_prefix() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Orokin"] = {
                Name = "Orokin",
                Description = "An ancient and powerful civilization.",
                Image = "Orokin.png"
            }
        }"#).unwrap();

        process_factions_data(&conn, &data).unwrap();

        let sigil_path: String = conn.query_row(
            "SELECT sigil_path FROM syndicates WHERE name = 'Orokin'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(sigil_path, "assets/factions/Orokin.png");
    }

    #[test]
    fn test_factions_no_image() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Sentient"] = {
                Name = "Sentient",
                Description = "Adaptive alien machines."
            }
        }"#).unwrap();

        let result = process_factions_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 1);
        assert_eq!(result.images.len(), 0);

        let sigil_path: String = conn.query_row(
            "SELECT sigil_path FROM syndicates WHERE name = 'Sentient'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(sigil_path, "");
    }
}
