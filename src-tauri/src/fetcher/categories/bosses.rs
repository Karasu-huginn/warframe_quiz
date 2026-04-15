use rusqlite::{params, Connection};
use serde_json::Value;
use crate::fetcher::{CategoryResult, CategoryReport, ImageTask};
use crate::fetcher::wiki_client::WikiClient;
use crate::fetcher::lua_parser;

const ENEMY_MODULES: &[(&str, &str)] = &[
    ("Module:Enemies/data/corpus", "Corpus"),
    ("Module:Enemies/data/grineer", "Grineer"),
    ("Module:Enemies/data/infestation", "Infested"),
    ("Module:Enemies/data/orokin", "Orokin"),
    ("Module:Enemies/data/sentient", "Sentient"),
    ("Module:Enemies/data/narmer", "Narmer"),
    ("Module:Enemies/data/unaffiliated", "Unaffiliated"),
    ("Module:Enemies/data/themurmur", "Murmur"),
    ("Module:Enemies/data/techrot", "Techrot"),
    ("Module:Enemies/data/scaldra", "Scaldra"),
    ("Module:Enemies/data/stalker", "Stalker"),
];

pub fn fetch_bosses(conn: &Connection, wiki: &WikiClient) -> Result<CategoryResult, String> {
    let mut combined_report = CategoryReport { category: "bosses".to_string(), ..Default::default() };
    let mut combined_images = Vec::new();

    for (module, faction) in ENEMY_MODULES {
        let source = wiki.fetch_module_source(module)?;
        let data = lua_parser::eval_lua_module(&source)?;
        let result = process_bosses_data(conn, &data, faction)?;
        combined_report.inserted += result.report.inserted;
        combined_report.failed += result.report.failed;
        combined_images.extend(result.images);
    }

    Ok(CategoryResult { report: combined_report, images: combined_images })
}

/// Determines whether a Lua entry represents a boss rather than a regular enemy.
/// Keeps entries that have BossLocation, AssassinationTarget, or Planet fields,
/// or whose Type field contains "Boss".
fn is_boss_entry(entry: &Value) -> bool {
    if !entry["BossLocation"].is_null() {
        return true;
    }
    if !entry["AssassinationTarget"].is_null() {
        return true;
    }
    if !entry["Planet"].is_null() {
        return true;
    }
    if let Some(type_str) = entry["Type"].as_str() {
        if type_str.to_lowercase().contains("boss") {
            return true;
        }
    }
    false
}

pub fn process_bosses_data(conn: &Connection, data: &Value, faction: &str) -> Result<CategoryResult, String> {
    let entries = data.as_object().ok_or("bosses data: expected object")?;
    let mut report = CategoryReport { category: "bosses".to_string(), ..Default::default() };
    let mut images = Vec::new();

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    for (_key, entry) in entries {
        let name = match entry["Name"].as_str() {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };

        if !is_boss_entry(entry) {
            continue;
        }

        // Planet comes from BossLocation, then Planet field
        let planet = entry["BossLocation"].as_str()
            .or_else(|| entry["Planet"].as_str())
            .unwrap_or_default();

        let description = entry["Description"].as_str().unwrap_or_default();
        let image = entry["Image"].as_str().map(|s| s.to_string());
        let icon_path = image.as_ref().map(|img| format!("assets/bosses/{img}")).unwrap_or_default();

        match tx.execute(
            "INSERT INTO bosses (name, planet, faction, description, icon_path)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![name, planet, faction, description, icon_path],
        ) {
            Ok(_) => report.inserted += 1,
            Err(e) => {
                eprintln!("Failed to insert boss {name}: {e}");
                report.failed += 1;
            }
        }

        if let Some(img) = image {
            images.push(ImageTask { wiki_filename: img, local_subdir: "bosses".to_string() });
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
    fn test_process_bosses_basic() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Jackal"] = {
                Name = "Jackal",
                Type = "Boss",
                BossLocation = "Venus",
                Description = "A Corpus spider tank.",
                Image = "Jackal.png"
            }
        }"#).unwrap();

        let result = process_bosses_data(&conn, &data, "Corpus").unwrap();
        assert_eq!(result.report.inserted, 1);
        assert_eq!(result.report.failed, 0);
        assert_eq!(result.images.len(), 1);
        assert_eq!(result.images[0].wiki_filename, "Jackal.png");
        assert_eq!(result.images[0].local_subdir, "bosses");

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM bosses", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        let (name, faction, planet): (String, String, String) = conn.query_row(
            "SELECT name, faction, planet FROM bosses WHERE name = 'Jackal'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).unwrap();
        assert_eq!(name, "Jackal");
        assert_eq!(faction, "Corpus");
        assert_eq!(planet, "Venus");
    }

    #[test]
    fn test_bosses_filters_regular_enemies() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Crewman"] = {
                Name = "Crewman",
                Type = "Unit",
                Description = "A standard Corpus soldier."
            },
            ["Raptors"] = {
                Name = "Raptors",
                Type = "Boss",
                BossLocation = "Europa",
                Description = "Corpus flying boss.",
                Image = "Raptors.png"
            }
        }"#).unwrap();

        let result = process_bosses_data(&conn, &data, "Corpus").unwrap();
        // Only Raptors should be inserted, Crewman has no boss fields
        assert_eq!(result.report.inserted, 1);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM bosses", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        let name: String = conn.query_row(
            "SELECT name FROM bosses",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(name, "Raptors");
    }

    #[test]
    fn test_bosses_planet_field_fallback() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Tyl Regor"] = {
                Name = "Tyl Regor",
                Type = "Boss",
                Planet = "Uranus",
                Description = "A Grineer scientist.",
                Image = "TylRegor.png"
            }
        }"#).unwrap();

        process_bosses_data(&conn, &data, "Grineer").unwrap();

        let planet: String = conn.query_row(
            "SELECT planet FROM bosses WHERE name = 'Tyl Regor'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(planet, "Uranus");
    }

    #[test]
    fn test_bosses_assassination_target_field() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["The Sergeant"] = {
                Name = "The Sergeant",
                AssassinationTarget = true,
                Description = "A Corpus commander.",
                Image = "Sergeant.png"
            }
        }"#).unwrap();

        let result = process_bosses_data(&conn, &data, "Corpus").unwrap();
        assert_eq!(result.report.inserted, 1);
    }

    #[test]
    fn test_bosses_icon_path_prefix() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Lech Kril"] = {
                Name = "Lech Kril",
                Type = "Boss",
                BossLocation = "Mars",
                Description = "A Grineer general.",
                Image = "LechKril.png"
            }
        }"#).unwrap();

        process_bosses_data(&conn, &data, "Grineer").unwrap();

        let icon_path: String = conn.query_row(
            "SELECT icon_path FROM bosses WHERE name = 'Lech Kril'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(icon_path, "assets/bosses/LechKril.png");
    }

    #[test]
    fn test_bosses_faction_from_module() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Kela De Thaym"] = {
                Name = "Kela De Thaym",
                Type = "Boss",
                BossLocation = "Sedna",
                Description = "A Grineer gladiator.",
                Image = "KelaDeThaym.png"
            }
        }"#).unwrap();

        process_bosses_data(&conn, &data, "Grineer").unwrap();

        let faction: String = conn.query_row(
            "SELECT faction FROM bosses WHERE name = 'Kela De Thaym'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(faction, "Grineer");
    }
}
