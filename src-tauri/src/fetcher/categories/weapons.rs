use rusqlite::{params, Connection};
use serde_json::Value;
use crate::fetcher::{CategoryResult, CategoryReport, ImageTask};
use crate::fetcher::wiki_client::WikiClient;
use crate::fetcher::lua_parser;

const WEAPON_MODULES: &[(&str, &str)] = &[
    ("Module:Weapons/data/primary", "Primary"),
    ("Module:Weapons/data/secondary", "Secondary"),
    ("Module:Weapons/data/melee", "Melee"),
    ("Module:Weapons/data/archwing", "Archwing"),
    ("Module:Weapons/data/companion", "Companion"),
    ("Module:Weapons/data/railjack", "Railjack"),
    ("Module:Weapons/data/modular", "Modular"),
    ("Module:Weapons/data/misc", "Misc"),
];

pub fn fetch_weapons(conn: &Connection, wiki: &WikiClient) -> Result<CategoryResult, String> {
    let mut combined_report = CategoryReport { category: "weapons".to_string(), ..Default::default() };
    let mut combined_images = Vec::new();

    for (module, default_type) in WEAPON_MODULES {
        let source = wiki.fetch_module_source(module)?;
        let data = lua_parser::eval_lua_module(&source)?;
        let result = process_weapons_data(conn, &data, default_type)?;
        combined_report.inserted += result.report.inserted;
        combined_report.failed += result.report.failed;
        combined_images.extend(result.images);
    }

    Ok(CategoryResult { report: combined_report, images: combined_images })
}

pub fn process_weapons_data(conn: &Connection, data: &Value, default_type: &str) -> Result<CategoryResult, String> {
    let entries = data.as_object().ok_or("weapons data: expected object")?;
    let mut report = CategoryReport { category: "weapons".to_string(), ..Default::default() };
    let mut images = Vec::new();

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    for (_key, entry) in entries {
        let name = match entry["Name"].as_str() {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };

        let weapon_type = entry["Type"].as_str().unwrap_or(default_type);
        let subtype = entry["Class"].as_str().unwrap_or_default();
        let mastery_rank = entry["Mastery"].as_i64().map(|v| v as i32).unwrap_or(0);

        // Damage: nested table with keys like "Impact", "Puncture", "Slash", etc.
        let (damage_total, damage_impact, damage_puncture, damage_slash) =
            parse_damage(&entry["Damage"]);

        let crit_chance = entry["CritChance"].as_f64();
        let crit_multiplier = entry["CritMultiplier"].as_f64();
        let status_chance = entry["StatusChance"].as_f64();
        let fire_rate = entry["FireRate"].as_f64();
        let magazine_size = entry["Magazine"].as_i64().map(|v| v as i32).unwrap_or(0);
        let reload_time = entry["Reload"].as_f64();
        let trigger_type = entry["Trigger"].as_str().unwrap_or_default();
        let riven_disposition = entry["Disposition"].as_i64().map(|v| v as i32).unwrap_or(0);
        let image = entry["Image"].as_str().map(|s| s.to_string());
        let icon_path = image.as_ref().map(|img| format!("assets/weapons/{img}")).unwrap_or_default();

        match tx.execute(
            "INSERT INTO weapons (name, type, subtype, mastery_rank, damage_total,
             damage_impact, damage_puncture, damage_slash, crit_chance, crit_multiplier,
             status_chance, fire_rate, magazine_size, reload_time, trigger_type,
             riven_disposition, icon_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
             ON CONFLICT(name) DO UPDATE SET
             type=excluded.type, subtype=excluded.subtype,
             mastery_rank=excluded.mastery_rank, damage_total=excluded.damage_total,
             damage_impact=excluded.damage_impact, damage_puncture=excluded.damage_puncture,
             damage_slash=excluded.damage_slash, crit_chance=excluded.crit_chance,
             crit_multiplier=excluded.crit_multiplier, status_chance=excluded.status_chance,
             fire_rate=excluded.fire_rate, magazine_size=excluded.magazine_size,
             reload_time=excluded.reload_time, trigger_type=excluded.trigger_type,
             riven_disposition=excluded.riven_disposition, icon_path=excluded.icon_path",
            params![
                name, weapon_type, subtype, mastery_rank, damage_total,
                damage_impact, damage_puncture, damage_slash, crit_chance, crit_multiplier,
                status_chance, fire_rate, magazine_size, reload_time, trigger_type,
                riven_disposition, icon_path
            ],
        ) {
            Ok(_) => report.inserted += 1,
            Err(e) => {
                eprintln!("Failed to upsert weapon {name}: {e}");
                report.failed += 1;
            }
        }

        if let Some(img) = image {
            images.push(ImageTask { wiki_filename: img, local_subdir: "weapons".to_string() });
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(CategoryResult { report, images })
}

/// Parse the Damage table: sum all numeric values, and extract specific damage types.
fn parse_damage(damage_val: &Value) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    match damage_val.as_object() {
        None => (None, None, None, None),
        Some(dmg_map) => {
            let mut total = 0.0;
            let mut impact = None;
            let mut puncture = None;
            let mut slash = None;

            for (key, val) in dmg_map {
                if let Some(v) = val.as_f64() {
                    total += v;
                    match key.as_str() {
                        "Impact" => impact = Some(v),
                        "Puncture" => puncture = Some(v),
                        "Slash" => slash = Some(v),
                        _ => {}
                    }
                }
            }

            if total == 0.0 {
                (None, impact, puncture, slash)
            } else {
                (Some(total), impact, puncture, slash)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;
    use crate::fetcher::lua_parser;

    #[test]
    fn test_process_weapons_basic() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Braton"] = {
                Name = "Braton", Type = "Primary", Class = "Rifle",
                Mastery = 0,
                Damage = { Impact = 7.5, Puncture = 15.0, Slash = 12.5 },
                CritChance = 0.12, CritMultiplier = 1.6,
                StatusChance = 0.08, FireRate = 7.5,
                Magazine = 45, Reload = 2.0,
                Trigger = "Auto", Disposition = 5,
                Image = "Braton.png"
            }
        }"#).unwrap();

        let result = process_weapons_data(&conn, &data, "Primary").unwrap();
        assert_eq!(result.report.inserted, 1);
        assert_eq!(result.report.failed, 0);
        assert_eq!(result.images.len(), 1);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM weapons", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_damage_total_is_sum() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Braton"] = {
                Name = "Braton", Type = "Primary", Class = "Rifle",
                Mastery = 0,
                Damage = { Impact = 7.5, Puncture = 15.0, Slash = 12.5 },
                Magazine = 45, Trigger = "Auto", Disposition = 5, Image = "Braton.png"
            }
        }"#).unwrap();

        process_weapons_data(&conn, &data, "Primary").unwrap();

        let (total, impact, puncture, slash): (f64, f64, f64, f64) = conn.query_row(
            "SELECT damage_total, damage_impact, damage_puncture, damage_slash FROM weapons WHERE name = 'Braton'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        ).unwrap();

        assert!((total - 35.0).abs() < 1e-9, "damage_total should be 35.0, got {total}");
        assert!((impact - 7.5).abs() < 1e-9);
        assert!((puncture - 15.0).abs() < 1e-9);
        assert!((slash - 12.5).abs() < 1e-9);
    }

    #[test]
    fn test_upsert_updates_existing_weapon() {
        let conn = test_db();

        let data1 = lua_parser::eval_lua_module(r#"return {
            ["Braton"] = {
                Name = "Braton", Type = "Primary", Class = "Rifle",
                Mastery = 0, Magazine = 45, Trigger = "Auto", Disposition = 5,
                Damage = { Impact = 7.5 }, Image = "Braton.png"
            }
        }"#).unwrap();
        process_weapons_data(&conn, &data1, "Primary").unwrap();

        let data2 = lua_parser::eval_lua_module(r#"return {
            ["Braton"] = {
                Name = "Braton", Type = "Primary", Class = "Rifle",
                Mastery = 2, Magazine = 60, Trigger = "Auto", Disposition = 4,
                Damage = { Impact = 10.0 }, Image = "Braton.png"
            }
        }"#).unwrap();
        process_weapons_data(&conn, &data2, "Primary").unwrap();

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM weapons", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        let (mr, mag): (i32, i32) = conn.query_row(
            "SELECT mastery_rank, magazine_size FROM weapons WHERE name = 'Braton'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert_eq!(mr, 2);
        assert_eq!(mag, 60);
    }

    #[test]
    fn test_default_type_used_when_no_type_field() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Skana"] = {
                Name = "Skana", Class = "Sword",
                Mastery = 0, Magazine = 0, Trigger = "Melee", Disposition = 5,
                Damage = { Slash = 50.0 }, Image = "Skana.png"
            }
        }"#).unwrap();

        process_weapons_data(&conn, &data, "Melee").unwrap();

        let weapon_type: String = conn.query_row(
            "SELECT type FROM weapons WHERE name = 'Skana'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(weapon_type, "Melee");
    }

    #[test]
    fn test_weapons_no_damage_table() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Lato"] = {
                Name = "Lato", Type = "Secondary", Class = "Pistol",
                Mastery = 0, Magazine = 15, Trigger = "Semi", Disposition = 5,
                Image = "Lato.png"
            }
        }"#).unwrap();

        let result = process_weapons_data(&conn, &data, "Secondary").unwrap();
        assert_eq!(result.report.inserted, 1);

        // damage_total should be NULL
        let total: Option<f64> = conn.query_row(
            "SELECT damage_total FROM weapons WHERE name = 'Lato'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert!(total.is_none());
    }

    #[test]
    fn test_parse_damage_helper() {
        let data = lua_parser::eval_lua_module(
            r#"return { Impact = 10.0, Puncture = 20.0, Slash = 30.0, Toxin = 5.0 }"#
        ).unwrap();
        let (total, impact, puncture, slash) = parse_damage(&data);
        assert!((total.unwrap() - 65.0).abs() < 1e-9);
        assert!((impact.unwrap() - 10.0).abs() < 1e-9);
        assert!((puncture.unwrap() - 20.0).abs() < 1e-9);
        assert!((slash.unwrap() - 30.0).abs() < 1e-9);
    }
}
