use rusqlite::{params, Connection};
use crate::game::question_types::*;
use super::shuffle_answers;

pub fn generate(
    conn: &Connection,
    question_id: u64,
    time_limit: Option<u32>,
) -> Result<(Question, StoredQuestion), String> {
    let (ability_name, _ability_icon, wf_id, wf_name, wf_type): (String, String, i64, String, String) = conn.query_row(
        "SELECT a.name, a.icon_path, w.id, w.name, w.type FROM abilities a
         JOIN warframes w ON a.warframe_id = w.id
         ORDER BY RANDOM() LIMIT 1",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).map_err(|e| format!("no ability with warframe: {e}"))?;

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
            question_id, question_type: "WarframeByAbility".to_string(),
            question_text: "À quelle Warframe appartient cette capacité ?".to_string(),
            clue: Clue::Text(ability_name), answers, time_limit,
        },
        StoredQuestion {
            question_id, question_type: "WarframeByAbility".to_string(),
            correct_answer_index: correct_index,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn setup(conn: &rusqlite::Connection) {
        for (name, t) in &[("Excalibur","Warframe"),("Mag","Warframe"),("Volt","Warframe"),("Frost","Warframe")] {
            conn.execute("INSERT INTO warframes (name, type) VALUES (?1, ?2)", params![name, t]).unwrap();
        }
        let id: i64 = conn.query_row("SELECT id FROM warframes WHERE name='Excalibur'", [], |r| r.get(0)).unwrap();
        conn.execute("INSERT INTO abilities (name, warframe_id, slot_index) VALUES (?1,?2,?3)", params!["Slash Dash", id, 1]).unwrap();
    }

    #[test]
    fn test_generate() {
        let conn = test_db();
        setup(&conn);
        let (q, s) = generate(&conn, 1, None).unwrap();
        assert_eq!(q.question_type, "WarframeByAbility");
        assert_eq!(q.answers.len(), 4);
        assert!(s.correct_answer_index < 4);
        assert_eq!(q.answers[s.correct_answer_index].text, "Excalibur");
        if let Clue::Text(ability) = &q.clue {
            assert_eq!(ability, "Slash Dash");
        } else { panic!("expected Text clue"); }
    }
}
