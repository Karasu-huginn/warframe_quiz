use rusqlite::{params, Connection};
use crate::game::question_types::*;
use super::shuffle_answers;

pub fn generate(
    conn: &Connection,
    question_id: u64,
    time_limit: Option<u32>,
) -> Result<(Question, StoredQuestion), String> {
    let (ability_name, school_id, school_name): (String, i64, String) = conn.query_row(
        "SELECT fa.name, fs.id, fs.name
         FROM focus_abilities fa
         JOIN focus_schools fs ON fa.school_id = fs.id
         ORDER BY RANDOM() LIMIT 1",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|e| format!("no focus ability found: {e}"))?;

    let wrongs: Vec<String> = conn.prepare(
        "SELECT name FROM focus_schools WHERE id != ?1 ORDER BY RANDOM() LIMIT 3"
    ).map_err(|e| e.to_string())?
    .query_map(params![school_id], |row| row.get(0))
    .map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    if wrongs.len() < 3 { return Err("not enough focus schools for wrong answers".to_string()); }

    let (answers, correct_index) = shuffle_answers(school_name, wrongs);

    Ok((
        Question {
            question_id, question_type: "FocusSchoolByAbility".to_string(),
            question_text: "À quelle école de Focus appartient cette capacité ?".to_string(),
            clue: Clue::Text(ability_name), answers, time_limit,
        },
        StoredQuestion {
            question_id, question_type: "FocusSchoolByAbility".to_string(),
            correct_answer_index: correct_index,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn setup(conn: &rusqlite::Connection) {
        for school in &["Madurai", "Vazarin", "Naramon", "Unairu"] {
            conn.execute(
                "INSERT INTO focus_schools (name) VALUES (?1)",
                params![school],
            ).unwrap();
        }
        // Link one ability to the first school (Madurai, id=1)
        conn.execute(
            "INSERT INTO focus_abilities (name, school_id) VALUES ('Phoenix Talons', 1)",
            [],
        ).unwrap();
    }

    #[test]
    fn test_generate() {
        let conn = test_db();
        setup(&conn);
        let (q, s) = generate(&conn, 1, None).unwrap();
        assert_eq!(q.question_type, "FocusSchoolByAbility");
        assert_eq!(q.answers.len(), 4);
        assert!(s.correct_answer_index < 4);
        if let Clue::Text(ability_name) = &q.clue {
            assert_eq!(ability_name, "Phoenix Talons");
        } else {
            panic!("expected Text clue");
        }
        assert_eq!(q.answers[s.correct_answer_index].text, "Madurai");
    }
}
