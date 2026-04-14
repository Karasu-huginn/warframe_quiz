use rusqlite::{Connection, OptionalExtension, Result, params};

use crate::db::models::Quote;

const COLS: &str = "
    id, character_id, quote_text, audio_path, context
";

fn row_to_quote(row: &rusqlite::Row) -> Result<Quote> {
    let audio_path: String = row.get(3)?;

    Ok(Quote {
        id: row.get(0)?,
        character_id: row.get(1)?,
        quote_text: row.get(2)?,
        audio_path: if audio_path.is_empty() { None } else { Some(audio_path) },
        context: row.get(4)?,
    })
}

pub fn insert_quote(conn: &Connection, q: &Quote) -> Result<i64> {
    conn.execute(
        "INSERT INTO quotes (
            character_id, quote_text, audio_path, context
        ) VALUES (
            ?1, ?2, ?3, ?4
        )",
        params![
            q.character_id,
            q.quote_text,
            q.audio_path.as_deref().unwrap_or(""),
            q.context,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_quote_by_id(conn: &Connection, id: i64) -> Result<Option<Quote>> {
    let sql = format!("SELECT {} FROM quotes WHERE id = ?1", COLS);
    conn.query_row(&sql, params![id], row_to_quote).optional()
}

pub fn get_quotes_by_character(conn: &Connection, character_id: i64) -> Result<Vec<Quote>> {
    let sql = format!(
        "SELECT {} FROM quotes WHERE character_id = ?1",
        COLS
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![character_id], row_to_quote)?;
    rows.collect()
}

pub fn get_quote_count(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM quotes", [], |row| row.get(0))
}

pub fn get_random_quote(conn: &Connection) -> Result<Option<Quote>> {
    let sql = format!("SELECT {} FROM quotes ORDER BY RANDOM() LIMIT 1", COLS);
    conn.query_row(&sql, [], row_to_quote).optional()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Character;
    use crate::db::queries::characters::insert_character;
    use crate::db::schema::test_db;

    fn sample_character(name: &str) -> Character {
        Character {
            id: 0,
            name: name.to_string(),
            description: "Test".to_string(),
            faction: "Tenno".to_string(),
            location: "Orbiter".to_string(),
            role: "Guide".to_string(),
            voice_actor: None,
            icon_path: None,
        }
    }

    fn sample_quote(character_id: i64, text: &str) -> Quote {
        Quote {
            id: 0,
            character_id,
            quote_text: text.to_string(),
            audio_path: None,
            context: "Test context".to_string(),
        }
    }

    #[test]
    fn test_insert_and_get() {
        let conn = test_db();
        let char_id = insert_character(&conn, &sample_character("Ordis")).unwrap();
        let q = sample_quote(char_id, "Operator, I detected an anomaly!");
        let id = insert_quote(&conn, &q).unwrap();
        let got = get_quote_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(got.quote_text, "Operator, I detected an anomaly!");
        assert_eq!(got.character_id, char_id);
        assert_eq!(got.context, "Test context");
        assert_eq!(got.audio_path, None);
    }

    #[test]
    fn test_get_nonexistent() {
        let conn = test_db();
        let result = get_quote_by_id(&conn, 9999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_by_character() {
        let conn = test_db();
        let char1_id = insert_character(&conn, &sample_character("Ordis")).unwrap();
        let mut char2 = sample_character("Lotus");
        char2.name = "Lotus".to_string();
        let char2_id = insert_character(&conn, &char2).unwrap();

        insert_quote(&conn, &sample_quote(char1_id, "Quote A")).unwrap();
        insert_quote(&conn, &sample_quote(char1_id, "Quote B")).unwrap();
        insert_quote(&conn, &sample_quote(char2_id, "Quote C")).unwrap();

        let results = get_quotes_by_character(&conn, char1_id).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|q| q.character_id == char1_id));
    }

    #[test]
    fn test_count_and_random() {
        let conn = test_db();
        assert_eq!(get_quote_count(&conn).unwrap(), 0);
        assert!(get_random_quote(&conn).unwrap().is_none());

        let char_id = insert_character(&conn, &sample_character("Ordis")).unwrap();
        insert_quote(&conn, &sample_quote(char_id, "Quote 1")).unwrap();
        assert_eq!(get_quote_count(&conn).unwrap(), 1);

        insert_quote(&conn, &sample_quote(char_id, "Quote 2")).unwrap();
        assert_eq!(get_quote_count(&conn).unwrap(), 2);

        let random = get_random_quote(&conn).unwrap();
        assert!(random.is_some());
        assert_eq!(random.unwrap().character_id, char_id);
    }
}
