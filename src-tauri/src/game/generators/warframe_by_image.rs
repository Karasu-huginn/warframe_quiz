use rusqlite::{params, Connection};
use crate::game::question_types::*;
use super::shuffle_answers;

pub fn generate(
    conn: &Connection,
    question_id: u64,
    time_limit: Option<u32>,
) -> Result<(Question, StoredQuestion), String> {
    let (wf_id, wf_name, wf_type, icon_path): (i64, String, String, String) = conn.query_row(
        "SELECT id, name, type, icon_path FROM warframes
         WHERE icon_path IS NOT NULL AND icon_path != ''
         ORDER BY RANDOM() LIMIT 1",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).map_err(|e| format!("no warframe with icon: {e}"))?;

    // Type-matched wrong answers with fallback
    let mut wrongs: Vec<String> = conn.prepare(
        "SELECT name FROM warframes WHERE type = ?1 AND id != ?2 ORDER BY RANDOM() LIMIT 3"
    ).map_err(|e| e.to_string())?
    .query_map(params![wf_type, wf_id], |row| row.get(0))
    .map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    if wrongs.len() < 3 {
        let more: Vec<String> = conn.prepare(
            "SELECT name FROM warframes WHERE id != ?1 ORDER BY RANDOM() LIMIT ?2"
        ).map_err(|e| e.to_string())?
        .query_map(params![wf_id, (3 - wrongs.len()) as i64], |row| row.get(0))
        .map_err(|e| e.to_string())?.filter_map(|r| r.ok())
        .filter(|n| !wrongs.contains(n)).collect();
        wrongs.extend(more);
    }

    if wrongs.len() < 3 { return Err("not enough warframes".to_string()); }

    let (answers, correct_index) = shuffle_answers(wf_name, wrongs);

    Ok((
        Question {
            question_id, question_type: "WarframeByImage".to_string(),
            question_text: "Quelle est cette Warframe ?".to_string(),
            clue: Clue::Image(icon_path), answers, time_limit,
        },
        StoredQuestion {
            question_id, question_type: "WarframeByImage".to_string(),
            correct_answer_index: correct_index,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn setup(conn: &rusqlite::Connection) {
        for (name, t, icon) in &[
            ("Excalibur","Warframe","img/excalibur.png"),
            ("Mag","Warframe","img/mag.png"),
            ("Volt","Warframe","img/volt.png"),
            ("Frost","Warframe","img/frost.png"),
        ] {
            conn.execute(
                "INSERT INTO warframes (name, type, icon_path) VALUES (?1, ?2, ?3)",
                params![name, t, icon],
            ).unwrap();
        }
    }

    #[test]
    fn test_generate() {
        let conn = test_db();
        setup(&conn);
        let (q, s) = generate(&conn, 1, None).unwrap();
        assert_eq!(q.question_type, "WarframeByImage");
        assert_eq!(q.answers.len(), 4);
        assert!(s.correct_answer_index < 4);
        if let Clue::Image(path) = &q.clue {
            assert!(!path.is_empty());
        } else { panic!("expected Image clue"); }
    }
}
