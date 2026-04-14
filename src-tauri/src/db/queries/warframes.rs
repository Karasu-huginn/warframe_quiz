use rusqlite::{Connection, OptionalExtension, Result, params};

use crate::db::models::Warframe;

const COLS: &str = "
    id, name, type, description,
    health, shields, armor, energy, sprint_speed,
    passive, mastery_rank, acquisition,
    release_date, prime_variant, is_vaulted,
    helminth_ability, sex, icon_path
";

fn row_to_warframe(row: &rusqlite::Row) -> Result<Warframe> {
    let release_date: String = row.get(12)?;
    let prime_variant: String = row.get(13)?;
    let helminth_ability: String = row.get(15)?;
    let sex: String = row.get(16)?;
    let icon_path: String = row.get(17)?;

    Ok(Warframe {
        id: row.get(0)?,
        name: row.get(1)?,
        wf_type: row.get(2)?,
        description: row.get(3)?,
        health: row.get(4)?,
        shields: row.get(5)?,
        armor: row.get(6)?,
        energy: row.get(7)?,
        sprint_speed: row.get(8)?,
        passive: row.get(9)?,
        mastery_rank: row.get(10)?,
        acquisition: row.get(11)?,
        release_date: if release_date.is_empty() { None } else { Some(release_date) },
        prime_variant: if prime_variant.is_empty() { None } else { Some(prime_variant) },
        is_vaulted: row.get(14)?,
        helminth_ability: if helminth_ability.is_empty() { None } else { Some(helminth_ability) },
        sex: if sex.is_empty() { None } else { Some(sex) },
        icon_path: if icon_path.is_empty() { None } else { Some(icon_path) },
    })
}

pub fn insert_warframe(conn: &Connection, wf: &Warframe) -> Result<i64> {
    conn.execute(
        "INSERT INTO warframes (
            name, type, description,
            health, shields, armor, energy, sprint_speed,
            passive, mastery_rank, acquisition,
            release_date, prime_variant, is_vaulted,
            helminth_ability, sex, icon_path
        ) VALUES (
            ?1, ?2, ?3,
            ?4, ?5, ?6, ?7, ?8,
            ?9, ?10, ?11,
            ?12, ?13, ?14,
            ?15, ?16, ?17
        )",
        params![
            wf.name,
            wf.wf_type,
            wf.description,
            wf.health,
            wf.shields,
            wf.armor,
            wf.energy,
            wf.sprint_speed,
            wf.passive,
            wf.mastery_rank,
            wf.acquisition,
            wf.release_date.as_deref().unwrap_or(""),
            wf.prime_variant.as_deref().unwrap_or(""),
            wf.is_vaulted,
            wf.helminth_ability.as_deref().unwrap_or(""),
            wf.sex.as_deref().unwrap_or(""),
            wf.icon_path.as_deref().unwrap_or(""),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_warframe_by_id(conn: &Connection, id: i64) -> Result<Option<Warframe>> {
    let sql = format!("SELECT {} FROM warframes WHERE id = ?1", COLS);
    conn.query_row(&sql, params![id], row_to_warframe).optional()
}

pub fn get_warframe_count(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM warframes", [], |row| row.get(0))
}

pub fn get_random_warframes(
    conn: &Connection,
    count: usize,
    exclude_id: Option<i64>,
    type_filter: Option<&str>,
) -> Result<Vec<Warframe>> {
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
        "SELECT {} FROM warframes {} ORDER BY RANDOM() LIMIT {}",
        COLS, where_clause, count
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_warframe)?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn sample(name: &str, wf_type: &str) -> Warframe {
        Warframe {
            id: 0,
            name: name.to_string(),
            wf_type: wf_type.to_string(),
            description: "Test".to_string(),
            health: Some(100.0),
            shields: Some(100.0),
            armor: Some(200.0),
            energy: Some(100.0),
            sprint_speed: Some(1.0),
            passive: "Test".to_string(),
            mastery_rank: Some(0),
            acquisition: "Market".to_string(),
            release_date: None,
            prime_variant: None,
            is_vaulted: false,
            helminth_ability: None,
            sex: Some("Male".to_string()),
            icon_path: None,
        }
    }

    #[test]
    fn test_insert_and_get() {
        let conn = test_db();
        let wf = sample("Excalibur", "Warframe");
        let id = insert_warframe(&conn, &wf).unwrap();
        let got = get_warframe_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(got.name, "Excalibur");
        assert_eq!(got.wf_type, "Warframe");
        assert_eq!(got.description, "Test");
        assert_eq!(got.health, Some(100.0));
        assert_eq!(got.is_vaulted, false);
        assert_eq!(got.sex, Some("Male".to_string()));
        assert_eq!(got.release_date, None);
    }

    #[test]
    fn test_get_nonexistent() {
        let conn = test_db();
        let result = get_warframe_by_id(&conn, 9999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_count() {
        let conn = test_db();
        assert_eq!(get_warframe_count(&conn).unwrap(), 0);
        insert_warframe(&conn, &sample("Volt", "Warframe")).unwrap();
        assert_eq!(get_warframe_count(&conn).unwrap(), 1);
        insert_warframe(&conn, &sample("Mag", "Warframe")).unwrap();
        assert_eq!(get_warframe_count(&conn).unwrap(), 2);
    }

    #[test]
    fn test_random_excludes_id() {
        let conn = test_db();
        let id1 = insert_warframe(&conn, &sample("Volt", "Warframe")).unwrap();
        insert_warframe(&conn, &sample("Mag", "Warframe")).unwrap();
        insert_warframe(&conn, &sample("Ash", "Warframe")).unwrap();

        let results = get_random_warframes(&conn, 2, Some(id1), None).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|w| w.id != id1));
    }

    #[test]
    fn test_random_type_filter() {
        let conn = test_db();
        insert_warframe(&conn, &sample("Volt", "Warframe")).unwrap();
        insert_warframe(&conn, &sample("Itzal", "Archwing")).unwrap();

        let results = get_random_warframes(&conn, 10, None, Some("Archwing")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Itzal");
        assert_eq!(results[0].wf_type, "Archwing");
    }
}
