// placeholder — full implementation below
use rusqlite::{Connection, OptionalExtension, Result, params};

use crate::db::models::Ability;

const COLS: &str = "
    id, name, cost, description, icon_path,
    warframe_id, slot_index, is_helminth, augment_mod_name
";

fn row_to_ability(row: &rusqlite::Row) -> Result<Ability> {
    let icon_path: String = row.get(4)?;
    let augment_mod_name: String = row.get(8)?;

    Ok(Ability {
        id: row.get(0)?,
        name: row.get(1)?,
        cost: row.get(2)?,
        description: row.get(3)?,
        icon_path: if icon_path.is_empty() { None } else { Some(icon_path) },
        warframe_id: row.get(5)?,
        slot_index: row.get(6)?,
        is_helminth: row.get(7)?,
        augment_mod_name: if augment_mod_name.is_empty() { None } else { Some(augment_mod_name) },
    })
}

pub fn insert_ability(conn: &Connection, a: &Ability) -> Result<i64> {
    conn.execute(
        "INSERT INTO abilities (
            name, cost, description, icon_path,
            warframe_id, slot_index, is_helminth, augment_mod_name
        ) VALUES (
            ?1, ?2, ?3, ?4,
            ?5, ?6, ?7, ?8
        )",
        params![
            a.name,
            a.cost,
            a.description,
            a.icon_path.as_deref().unwrap_or(""),
            a.warframe_id,
            a.slot_index,
            a.is_helminth,
            a.augment_mod_name.as_deref().unwrap_or(""),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_ability_by_id(conn: &Connection, id: i64) -> Result<Option<Ability>> {
    let sql = format!("SELECT {} FROM abilities WHERE id = ?1", COLS);
    conn.query_row(&sql, params![id], row_to_ability).optional()
}

pub fn get_abilities_by_warframe(conn: &Connection, warframe_id: i64) -> Result<Vec<Ability>> {
    let sql = format!(
        "SELECT {} FROM abilities WHERE warframe_id = ?1 ORDER BY slot_index",
        COLS
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![warframe_id], row_to_ability)?;
    rows.collect()
}

pub fn get_ability_count(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM abilities", [], |row| row.get(0))
}

pub fn get_random_abilities(
    conn: &Connection,
    count: usize,
    exclude_warframe_id: Option<i64>,
) -> Result<Vec<Ability>> {
    let where_clause = match exclude_warframe_id {
        Some(id) => format!("WHERE warframe_id != {}", id),
        None => String::new(),
    };

    let sql = format!(
        "SELECT {} FROM abilities {} ORDER BY RANDOM() LIMIT {}",
        COLS, where_clause, count
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_ability)?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Warframe;
    use crate::db::queries::warframes::insert_warframe;
    use crate::db::schema::test_db;

    fn sample_warframe() -> Warframe {
        Warframe {
            id: 0,
            name: "TestFrame".to_string(),
            wf_type: "Warframe".to_string(),
            description: "Test".to_string(),
            health: Some(100.0),
            shields: None,
            armor: None,
            energy: None,
            sprint_speed: None,
            passive: "Test".to_string(),
            mastery_rank: Some(0),
            acquisition: "Market".to_string(),
            release_date: None,
            prime_variant: None,
            is_vaulted: false,
            helminth_ability: None,
            sex: None,
            icon_path: None,
        }
    }

    fn sample_ability(name: &str, warframe_id: i64, slot: i32) -> Ability {
        Ability {
            id: 0,
            name: name.to_string(),
            cost: Some(25),
            description: "Test ability".to_string(),
            icon_path: None,
            warframe_id,
            slot_index: Some(slot),
            is_helminth: false,
            augment_mod_name: None,
        }
    }

    #[test]
    fn test_insert_and_get() {
        let conn = test_db();
        let wf_id = insert_warframe(&conn, &sample_warframe()).unwrap();
        let ab = sample_ability("Slash Dash", wf_id, 0);
        let id = insert_ability(&conn, &ab).unwrap();
        let got = get_ability_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(got.name, "Slash Dash");
        assert_eq!(got.cost, Some(25));
        assert_eq!(got.warframe_id, wf_id);
        assert_eq!(got.slot_index, Some(0));
        assert_eq!(got.is_helminth, false);
        assert_eq!(got.icon_path, None);
    }

    #[test]
    fn test_get_nonexistent() {
        let conn = test_db();
        let result = get_ability_by_id(&conn, 9999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_count() {
        let conn = test_db();
        assert_eq!(get_ability_count(&conn).unwrap(), 0);
        let wf_id = insert_warframe(&conn, &sample_warframe()).unwrap();
        insert_ability(&conn, &sample_ability("A", wf_id, 0)).unwrap();
        assert_eq!(get_ability_count(&conn).unwrap(), 1);
        insert_ability(&conn, &sample_ability("B", wf_id, 1)).unwrap();
        assert_eq!(get_ability_count(&conn).unwrap(), 2);
    }

    #[test]
    fn test_get_abilities_by_warframe() {
        let conn = test_db();
        let wf1_id = insert_warframe(&conn, &sample_warframe()).unwrap();
        let mut wf2 = sample_warframe();
        wf2.name = "OtherFrame".to_string();
        let wf2_id = insert_warframe(&conn, &wf2).unwrap();

        insert_ability(&conn, &sample_ability("Ability1", wf1_id, 0)).unwrap();
        insert_ability(&conn, &sample_ability("Ability2", wf1_id, 1)).unwrap();
        insert_ability(&conn, &sample_ability("OtherAbility", wf2_id, 0)).unwrap();

        let results = get_abilities_by_warframe(&conn, wf1_id).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|a| a.warframe_id == wf1_id));
        // Ordered by slot_index
        assert_eq!(results[0].slot_index, Some(0));
        assert_eq!(results[1].slot_index, Some(1));
    }

    #[test]
    fn test_random_excludes_warframe() {
        let conn = test_db();
        let wf1_id = insert_warframe(&conn, &sample_warframe()).unwrap();
        let mut wf2 = sample_warframe();
        wf2.name = "OtherFrame".to_string();
        let wf2_id = insert_warframe(&conn, &wf2).unwrap();

        insert_ability(&conn, &sample_ability("Ability1", wf1_id, 0)).unwrap();
        insert_ability(&conn, &sample_ability("Ability2", wf1_id, 1)).unwrap();
        insert_ability(&conn, &sample_ability("OtherAbility", wf2_id, 0)).unwrap();

        let results = get_random_abilities(&conn, 10, Some(wf1_id)).unwrap();
        assert!(results.iter().all(|a| a.warframe_id != wf1_id));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].warframe_id, wf2_id);
    }
}
