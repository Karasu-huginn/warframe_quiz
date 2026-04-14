use rusqlite::{Connection, OptionalExtension, Result, params};

use crate::db::models::Mod;

const COLS: &str = "
    id, name, polarity, rarity, mod_type,
    max_rank, base_drain, effect_description,
    set_name, is_exilus, is_augment, augment_warframe_id, icon_path
";

fn row_to_mod(row: &rusqlite::Row) -> Result<Mod> {
    let polarity: String = row.get(2)?;
    let rarity: String = row.get(3)?;
    let mod_type: String = row.get(4)?;
    let set_name: String = row.get(8)?;
    let icon_path: String = row.get(12)?;

    Ok(Mod {
        id: row.get(0)?,
        name: row.get(1)?,
        polarity: if polarity.is_empty() { None } else { Some(polarity) },
        rarity: if rarity.is_empty() { None } else { Some(rarity) },
        mod_type: if mod_type.is_empty() { None } else { Some(mod_type) },
        max_rank: row.get(5)?,
        base_drain: row.get(6)?,
        effect_description: row.get(7)?,
        set_name: if set_name.is_empty() { None } else { Some(set_name) },
        is_exilus: row.get(9)?,
        is_augment: row.get(10)?,
        augment_warframe_id: row.get(11)?,
        icon_path: if icon_path.is_empty() { None } else { Some(icon_path) },
    })
}

pub fn insert_mod(conn: &Connection, m: &Mod) -> Result<i64> {
    conn.execute(
        "INSERT INTO mods (
            name, polarity, rarity, mod_type,
            max_rank, base_drain, effect_description,
            set_name, is_exilus, is_augment, augment_warframe_id, icon_path
        ) VALUES (
            ?1, ?2, ?3, ?4,
            ?5, ?6, ?7,
            ?8, ?9, ?10, ?11, ?12
        )",
        params![
            m.name,
            m.polarity.as_deref().unwrap_or(""),
            m.rarity.as_deref().unwrap_or(""),
            m.mod_type.as_deref().unwrap_or(""),
            m.max_rank,
            m.base_drain,
            m.effect_description,
            m.set_name.as_deref().unwrap_or(""),
            m.is_exilus,
            m.is_augment,
            m.augment_warframe_id,
            m.icon_path.as_deref().unwrap_or(""),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_mod_by_id(conn: &Connection, id: i64) -> Result<Option<Mod>> {
    let sql = format!("SELECT {} FROM mods WHERE id = ?1", COLS);
    conn.query_row(&sql, params![id], row_to_mod).optional()
}

pub fn get_mod_count(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM mods", [], |row| row.get(0))
}

pub fn get_random_mods(
    conn: &Connection,
    count: usize,
    exclude_id: Option<i64>,
    mod_type_filter: Option<&str>,
) -> Result<Vec<Mod>> {
    let mut conditions: Vec<String> = Vec::new();

    if let Some(id) = exclude_id {
        conditions.push(format!("id != {}", id));
    }
    if let Some(t) = mod_type_filter {
        conditions.push(format!("mod_type = '{}'", t));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT {} FROM mods {} ORDER BY RANDOM() LIMIT {}",
        COLS, where_clause, count
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_mod)?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn sample(name: &str, mod_type: &str) -> Mod {
        Mod {
            id: 0,
            name: name.to_string(),
            polarity: Some("Madurai".to_string()),
            rarity: Some("Rare".to_string()),
            mod_type: Some(mod_type.to_string()),
            max_rank: Some(5),
            base_drain: Some(6),
            effect_description: "Test effect".to_string(),
            set_name: None,
            is_exilus: false,
            is_augment: false,
            augment_warframe_id: None,
            icon_path: None,
        }
    }

    #[test]
    fn test_insert_and_get() {
        let conn = test_db();
        let m = sample("Serration", "Rifle");
        let id = insert_mod(&conn, &m).unwrap();
        let got = get_mod_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(got.name, "Serration");
        assert_eq!(got.mod_type, Some("Rifle".to_string()));
        assert_eq!(got.polarity, Some("Madurai".to_string()));
        assert_eq!(got.rarity, Some("Rare".to_string()));
        assert_eq!(got.max_rank, Some(5));
        assert_eq!(got.is_exilus, false);
        assert_eq!(got.is_augment, false);
        assert_eq!(got.icon_path, None);
        assert_eq!(got.set_name, None);
    }

    #[test]
    fn test_get_nonexistent() {
        let conn = test_db();
        let result = get_mod_by_id(&conn, 9999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_count() {
        let conn = test_db();
        assert_eq!(get_mod_count(&conn).unwrap(), 0);
        insert_mod(&conn, &sample("Serration", "Rifle")).unwrap();
        assert_eq!(get_mod_count(&conn).unwrap(), 1);
        insert_mod(&conn, &sample("Hornet Strike", "Pistol")).unwrap();
        assert_eq!(get_mod_count(&conn).unwrap(), 2);
    }

    #[test]
    fn test_random_mod_type_filter() {
        let conn = test_db();
        insert_mod(&conn, &sample("Serration", "Rifle")).unwrap();
        insert_mod(&conn, &sample("Hornet Strike", "Pistol")).unwrap();
        insert_mod(&conn, &sample("Pressure Point", "Melee")).unwrap();

        let results = get_random_mods(&conn, 10, None, Some("Pistol")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Hornet Strike");
        assert_eq!(results[0].mod_type, Some("Pistol".to_string()));
    }

    #[test]
    fn test_random_excludes_id() {
        let conn = test_db();
        let id1 = insert_mod(&conn, &sample("Serration", "Rifle")).unwrap();
        insert_mod(&conn, &sample("Split Chamber", "Rifle")).unwrap();
        insert_mod(&conn, &sample("Point Strike", "Rifle")).unwrap();

        let results = get_random_mods(&conn, 10, Some(id1), None).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|m| m.id != id1));
    }
}
