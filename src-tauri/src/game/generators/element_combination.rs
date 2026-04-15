use rusqlite::{params, Connection};
use crate::game::question_types::*;
use super::shuffle_answers;

pub fn generate(
    conn: &Connection,
    question_id: u64,
    time_limit: Option<u32>,
) -> Result<(Question, StoredQuestion), String> {
    let (element_id, element_name, component_a, component_b): (i64, String, String, String) = conn.query_row(
        "SELECT id, name, component_a, component_b FROM elements
         WHERE element_type = 'combined' AND component_a != '' AND component_b != ''
         ORDER BY RANDOM() LIMIT 1",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).map_err(|e| format!("no combined element found: {e}"))?;

    let wrongs: Vec<String> = conn.prepare(
        "SELECT name FROM elements WHERE element_type = 'combined' AND id != ?1 ORDER BY RANDOM() LIMIT 3"
    ).map_err(|e| e.to_string())?
    .query_map(params![element_id], |row| row.get(0))
    .map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    if wrongs.len() < 3 { return Err("not enough combined elements for wrong answers".to_string()); }

    let (answers, correct_index) = shuffle_answers(element_name, wrongs);

    Ok((
        Question {
            question_id, question_type: "ElementCombination".to_string(),
            question_text: "Quel élément résulte de cette combinaison ?".to_string(),
            clue: Clue::TwoElements { element_a: component_a, element_b: component_b },
            answers, time_limit,
        },
        StoredQuestion {
            question_id, question_type: "ElementCombination".to_string(),
            correct_answer_index: correct_index,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn setup(conn: &rusqlite::Connection) {
        for (name, comp_a, comp_b) in &[
            ("Blast",     "Heat",     "Cold"),
            ("Corrosive", "Toxin",    "Electricity"),
            ("Gas",       "Heat",     "Toxin"),
            ("Magnetic",  "Cold",     "Electricity"),
        ] {
            conn.execute(
                "INSERT INTO elements (name, element_type, component_a, component_b)
                 VALUES (?1, 'combined', ?2, ?3)",
                params![name, comp_a, comp_b],
            ).unwrap();
        }
    }

    #[test]
    fn test_generate() {
        let conn = test_db();
        setup(&conn);
        let (q, s) = generate(&conn, 1, None).unwrap();
        assert_eq!(q.question_type, "ElementCombination");
        assert_eq!(q.answers.len(), 4);
        assert!(s.correct_answer_index < 4);
        if let Clue::TwoElements { element_a, element_b } = &q.clue {
            assert!(!element_a.is_empty());
            assert!(!element_b.is_empty());
        } else {
            panic!("expected TwoElements clue");
        }
        // Verify the correct answer text matches one of the inserted element names
        let correct_text = &q.answers[s.correct_answer_index].text;
        let valid_names = ["Blast", "Corrosive", "Gas", "Magnetic"];
        assert!(valid_names.contains(&correct_text.as_str()), "unexpected answer: {correct_text}");
    }
}
