use rusqlite::{params, Connection};
use crate::game::question_types::*;
use super::shuffle_answers;

pub fn generate(
    conn: &Connection,
    question_id: u64,
    time_limit: Option<u32>,
) -> Result<(Question, StoredQuestion), String> {
    let (resource_name, planet_id, planet_name): (String, i64, String) = conn.query_row(
        "SELECT pr.resource_name, p.id, p.name
         FROM planet_resources pr
         JOIN planets p ON pr.planet_id = p.id
         ORDER BY RANDOM() LIMIT 1",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|e| format!("no planet resource found: {e}"))?;

    let mut wrongs: Vec<String> = conn.prepare(
        "SELECT name FROM planets WHERE id != ?1 ORDER BY RANDOM() LIMIT 3"
    ).map_err(|e| e.to_string())?
    .query_map(params![planet_id], |row| row.get(0))
    .map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    if wrongs.len() < 3 { return Err("not enough planets for wrong answers".to_string()); }

    let (answers, correct_index) = shuffle_answers(planet_name, wrongs);

    Ok((
        Question {
            question_id, question_type: "PlanetByResource".to_string(),
            question_text: "Sur quelle planète trouve-t-on cette ressource ?".to_string(),
            clue: Clue::Text(resource_name), answers, time_limit,
        },
        StoredQuestion {
            question_id, question_type: "PlanetByResource".to_string(),
            correct_answer_index: correct_index,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn setup(conn: &rusqlite::Connection) {
        for name in &["Earth", "Mars", "Venus", "Mercury"] {
            conn.execute(
                "INSERT INTO planets (name) VALUES (?1)",
                params![name],
            ).unwrap();
        }
        // Link one resource to the first planet (Earth, id=1)
        conn.execute(
            "INSERT INTO planet_resources (planet_id, resource_name) VALUES (1, 'Ferrite')",
            [],
        ).unwrap();
    }

    #[test]
    fn test_generate() {
        let conn = test_db();
        setup(&conn);
        let (q, s) = generate(&conn, 1, None).unwrap();
        assert_eq!(q.question_type, "PlanetByResource");
        assert_eq!(q.answers.len(), 4);
        assert!(s.correct_answer_index < 4);
        if let Clue::Text(resource) = &q.clue {
            assert_eq!(resource, "Ferrite");
        } else { panic!("expected Text clue"); }
        assert_eq!(q.answers[s.correct_answer_index].text, "Earth");
    }
}
