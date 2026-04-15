use rusqlite::{params, Connection};
use crate::game::question_types::*;
use super::shuffle_answers;

pub fn generate(
    conn: &Connection,
    question_id: u64,
    time_limit: Option<u32>,
) -> Result<(Question, StoredQuestion), String> {
    let (mod_id, mod_name, mod_type, correct_effect): (i64, String, String, String) = conn.query_row(
        "SELECT id, name, mod_type, effect_description FROM mods
         WHERE effect_description != '' ORDER BY RANDOM() LIMIT 1",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).map_err(|e| format!("no mod with effect_description: {e}"))?;

    // Type-matched wrong answers with fallback
    let mut wrongs: Vec<String> = conn.prepare(
        "SELECT effect_description FROM mods WHERE mod_type = ?1 AND id != ?2 AND effect_description != '' ORDER BY RANDOM() LIMIT 3"
    ).map_err(|e| e.to_string())?
    .query_map(params![mod_type, mod_id], |row| row.get(0))
    .map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    if wrongs.len() < 3 {
        let more: Vec<String> = conn.prepare(
            "SELECT effect_description FROM mods WHERE id != ?1 AND effect_description != '' ORDER BY RANDOM() LIMIT ?2"
        ).map_err(|e| e.to_string())?
        .query_map(params![mod_id, (3 - wrongs.len()) as i64], |row| row.get(0))
        .map_err(|e| e.to_string())?.filter_map(|r| r.ok())
        .filter(|e| !wrongs.contains(e)).collect();
        wrongs.extend(more);
    }

    if wrongs.len() < 3 { return Err("not enough mods with effect descriptions".to_string()); }

    let (answers, correct_index) = shuffle_answers(correct_effect, wrongs);

    Ok((
        Question {
            question_id, question_type: "ModByEffect".to_string(),
            question_text: "Quel est l'effet de ce mod ?".to_string(),
            clue: Clue::Text(mod_name), answers, time_limit,
        },
        StoredQuestion {
            question_id, question_type: "ModByEffect".to_string(),
            correct_answer_index: correct_index,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn setup(conn: &rusqlite::Connection) {
        for (name, mod_type, effect) in &[
            ("Serration", "Rifle", "+165% Damage"),
            ("Point Strike", "Rifle", "+150% Critical Chance"),
            ("Speed Trigger", "Rifle", "+60% Fire Rate"),
            ("Vital Sense", "Rifle", "+120% Critical Damage"),
        ] {
            conn.execute(
                "INSERT INTO mods (name, mod_type, effect_description) VALUES (?1, ?2, ?3)",
                params![name, mod_type, effect],
            ).unwrap();
        }
    }

    #[test]
    fn test_generate() {
        let conn = test_db();
        setup(&conn);
        let (q, s) = generate(&conn, 1, None).unwrap();
        assert_eq!(q.question_type, "ModByEffect");
        assert_eq!(q.answers.len(), 4);
        assert!(s.correct_answer_index < 4);
        if let Clue::Text(mod_name) = &q.clue {
            assert!(!mod_name.is_empty());
        } else { panic!("expected Text clue"); }
        // The correct answer should be a non-empty effect description
        assert!(!q.answers[s.correct_answer_index].text.is_empty());
    }
}
