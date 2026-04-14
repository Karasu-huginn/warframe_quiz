use rusqlite::{Connection, OptionalExtension, Result, params};

use crate::db::models::Weapon;

const COLS: &str = "
    id, name, type, subtype, mastery_rank,
    damage_total, damage_impact, damage_puncture, damage_slash,
    crit_chance, crit_multiplier, status_chance,
    fire_rate, magazine_size, reload_time,
    trigger_type, noise_level, riven_disposition,
    acquisition, variant_type, base_weapon_id, release_date, icon_path
";

fn row_to_weapon(row: &rusqlite::Row) -> Result<Weapon> {
    let trigger_type: String = row.get(15)?;
    let noise_level: String = row.get(16)?;
    let acquisition: String = row.get(18)?;
    let variant_type: String = row.get(19)?;
    let release_date: String = row.get(21)?;
    let icon_path: String = row.get(22)?;

    Ok(Weapon {
        id: row.get(0)?,
        name: row.get(1)?,
        weapon_type: row.get(2)?,
        subtype: row.get(3)?,
        mastery_rank: row.get(4)?,
        damage_total: row.get(5)?,
        damage_impact: row.get(6)?,
        damage_puncture: row.get(7)?,
        damage_slash: row.get(8)?,
        crit_chance: row.get(9)?,
        crit_multiplier: row.get(10)?,
        status_chance: row.get(11)?,
        fire_rate: row.get(12)?,
        magazine_size: row.get(13)?,
        reload_time: row.get(14)?,
        trigger_type: if trigger_type.is_empty() { None } else { Some(trigger_type) },
        noise_level: if noise_level.is_empty() { None } else { Some(noise_level) },
        riven_disposition: {
            let rd: i64 = row.get(17)?;
            if rd == 0 { None } else { Some(rd as f64) }
        },
        acquisition: if acquisition.is_empty() { "".to_string() } else { acquisition },
        variant_type: if variant_type.is_empty() { None } else { Some(variant_type) },
        base_weapon_id: row.get(20)?,
        release_date: if release_date.is_empty() { None } else { Some(release_date) },
        icon_path: if icon_path.is_empty() { None } else { Some(icon_path) },
    })
}

pub fn insert_weapon(conn: &Connection, w: &Weapon) -> Result<i64> {
    conn.execute(
        "INSERT INTO weapons (
            name, type, subtype, mastery_rank,
            damage_total, damage_impact, damage_puncture, damage_slash,
            crit_chance, crit_multiplier, status_chance,
            fire_rate, magazine_size, reload_time,
            trigger_type, noise_level, riven_disposition,
            acquisition, variant_type, base_weapon_id, release_date, icon_path
        ) VALUES (
            ?1, ?2, ?3, ?4,
            ?5, ?6, ?7, ?8,
            ?9, ?10, ?11,
            ?12, ?13, ?14,
            ?15, ?16, ?17,
            ?18, ?19, ?20, ?21, ?22
        )",
        params![
            w.name,
            w.weapon_type,
            w.subtype,
            w.mastery_rank.unwrap_or(0),
            w.damage_total,
            w.damage_impact,
            w.damage_puncture,
            w.damage_slash,
            w.crit_chance,
            w.crit_multiplier,
            w.status_chance,
            w.fire_rate,
            w.magazine_size.unwrap_or(0),
            w.reload_time,
            w.trigger_type.as_deref().unwrap_or(""),
            w.noise_level.as_deref().unwrap_or(""),
            w.riven_disposition.unwrap_or(0.0),
            w.acquisition,
            w.variant_type.as_deref().unwrap_or(""),
            w.base_weapon_id,
            w.release_date.as_deref().unwrap_or(""),
            w.icon_path.as_deref().unwrap_or(""),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_weapon_by_id(conn: &Connection, id: i64) -> Result<Option<Weapon>> {
    let sql = format!("SELECT {} FROM weapons WHERE id = ?1", COLS);
    conn.query_row(&sql, params![id], row_to_weapon).optional()
}

pub fn get_weapon_count(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM weapons", [], |row| row.get(0))
}

pub fn get_random_weapons(
    conn: &Connection,
    count: usize,
    exclude_id: Option<i64>,
    type_filter: Option<&str>,
) -> Result<Vec<Weapon>> {
    let mut conditions: Vec<String> = Vec::new();

    if let Some(id) = exclude_id {
        conditions.push(format!("id != {}", id));
    }
    if let Some(t) = type_filter {
        conditions.push(format!("type = '{}'", t));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT {} FROM weapons {} ORDER BY RANDOM() LIMIT {}",
        COLS, where_clause, count
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_weapon)?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn sample(name: &str, weapon_type: &str) -> Weapon {
        Weapon {
            id: 0,
            name: name.to_string(),
            weapon_type: weapon_type.to_string(),
            subtype: "Sword".to_string(),
            mastery_rank: Some(0),
            damage_total: Some(100.0),
            damage_impact: Some(10.0),
            damage_puncture: Some(10.0),
            damage_slash: Some(80.0),
            crit_chance: Some(0.15),
            crit_multiplier: Some(1.5),
            status_chance: Some(0.10),
            fire_rate: Some(1.0),
            magazine_size: None,
            reload_time: None,
            trigger_type: Some("Auto".to_string()),
            noise_level: Some("Alarming".to_string()),
            riven_disposition: Some(1.0),
            acquisition: "Market".to_string(),
            variant_type: None,
            base_weapon_id: None,
            release_date: None,
            icon_path: None,
        }
    }

    #[test]
    fn test_insert_and_get() {
        let conn = test_db();
        let w = sample("Braton", "Primary");
        let id = insert_weapon(&conn, &w).unwrap();
        let got = get_weapon_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(got.name, "Braton");
        assert_eq!(got.weapon_type, "Primary");
        assert_eq!(got.damage_total, Some(100.0));
        assert_eq!(got.crit_chance, Some(0.15));
        assert_eq!(got.trigger_type, Some("Auto".to_string()));
        assert_eq!(got.release_date, None);
        assert_eq!(got.base_weapon_id, None);
        assert_eq!(got.mastery_rank, Some(0));
        // magazine_size None -> stored as 0 -> Some(0) due to NOT NULL DEFAULT 0 in schema
        assert_eq!(got.magazine_size, Some(0));
    }

    #[test]
    fn test_get_nonexistent() {
        let conn = test_db();
        let result = get_weapon_by_id(&conn, 9999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_count() {
        let conn = test_db();
        assert_eq!(get_weapon_count(&conn).unwrap(), 0);
        insert_weapon(&conn, &sample("Braton", "Primary")).unwrap();
        assert_eq!(get_weapon_count(&conn).unwrap(), 1);
        insert_weapon(&conn, &sample("Lato", "Secondary")).unwrap();
        assert_eq!(get_weapon_count(&conn).unwrap(), 2);
    }

    #[test]
    fn test_random_excludes_id() {
        let conn = test_db();
        let id1 = insert_weapon(&conn, &sample("Braton", "Primary")).unwrap();
        insert_weapon(&conn, &sample("Soma", "Primary")).unwrap();
        insert_weapon(&conn, &sample("Paris", "Primary")).unwrap();

        let results = get_random_weapons(&conn, 2, Some(id1), None).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|w| w.id != id1));
    }

    #[test]
    fn test_random_type_filter() {
        let conn = test_db();
        insert_weapon(&conn, &sample("Braton", "Primary")).unwrap();
        insert_weapon(&conn, &sample("Lato", "Secondary")).unwrap();
        insert_weapon(&conn, &sample("Skana", "Melee")).unwrap();

        let results = get_random_weapons(&conn, 10, None, Some("Melee")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Skana");
        assert_eq!(results[0].weapon_type, "Melee");
    }
}
