use rusqlite::{Connection, OptionalExtension, Result, params};

use crate::db::models::Character;

const COLS: &str = "
    id, name, description, faction,
    location, role, voice_actor, icon_path
";

fn row_to_character(row: &rusqlite::Row) -> Result<Character> {
    let voice_actor: String = row.get(6)?;
    let icon_path: String = row.get(7)?;

    Ok(Character {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        faction: row.get(3)?,
        location: row.get(4)?,
        role: row.get(5)?,
        voice_actor: if voice_actor.is_empty() { None } else { Some(voice_actor) },
        icon_path: if icon_path.is_empty() { None } else { Some(icon_path) },
    })
}

pub fn insert_character(conn: &Connection, c: &Character) -> Result<i64> {
    conn.execute(
        "INSERT INTO characters (
            name, description, faction,
            location, role, voice_actor, icon_path
        ) VALUES (
            ?1, ?2, ?3,
            ?4, ?5, ?6, ?7
        )",
        params![
            c.name,
            c.description,
            c.faction,
            c.location,
            c.role,
            c.voice_actor.as_deref().unwrap_or(""),
            c.icon_path.as_deref().unwrap_or(""),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_character_by_id(conn: &Connection, id: i64) -> Result<Option<Character>> {
    let sql = format!("SELECT {} FROM characters WHERE id = ?1", COLS);
    conn.query_row(&sql, params![id], row_to_character).optional()
}

pub fn get_character_count(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM characters", [], |row| row.get(0))
}

pub fn get_random_characters(
    conn: &Connection,
    count: usize,
    exclude_id: Option<i64>,
) -> Result<Vec<Character>> {
    let where_clause = match exclude_id {
        Some(id) => format!("WHERE id != {}", id),
        None => String::new(),
    };

    let sql = format!(
        "SELECT {} FROM characters {} ORDER BY RANDOM() LIMIT {}",
        COLS, where_clause, count
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_character)?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn sample(name: &str, faction: &str) -> Character {
        Character {
            id: 0,
            name: name.to_string(),
            description: "Test description".to_string(),
            faction: faction.to_string(),
            location: "Relay".to_string(),
            role: "Vendor".to_string(),
            voice_actor: None,
            icon_path: None,
        }
    }

    #[test]
    fn test_insert_and_get() {
        let conn = test_db();
        let c = sample("Teshin", "Conclave");
        let id = insert_character(&conn, &c).unwrap();
        let got = get_character_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(got.name, "Teshin");
        assert_eq!(got.faction, "Conclave");
        assert_eq!(got.description, "Test description");
        assert_eq!(got.location, "Relay");
        assert_eq!(got.role, "Vendor");
        assert_eq!(got.voice_actor, None);
        assert_eq!(got.icon_path, None);
    }

    #[test]
    fn test_get_nonexistent() {
        let conn = test_db();
        let result = get_character_by_id(&conn, 9999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_count() {
        let conn = test_db();
        assert_eq!(get_character_count(&conn).unwrap(), 0);
        insert_character(&conn, &sample("Teshin", "Conclave")).unwrap();
        assert_eq!(get_character_count(&conn).unwrap(), 1);
        insert_character(&conn, &sample("Cephalon Simaris", "Cephalon")).unwrap();
        assert_eq!(get_character_count(&conn).unwrap(), 2);
    }

    #[test]
    fn test_random_excludes_id() {
        let conn = test_db();
        let id1 = insert_character(&conn, &sample("Teshin", "Conclave")).unwrap();
        insert_character(&conn, &sample("Cephalon Simaris", "Cephalon")).unwrap();
        insert_character(&conn, &sample("Darvo", "Corpus")).unwrap();

        let results = get_random_characters(&conn, 10, Some(id1)).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|c| c.id != id1));
    }
}
