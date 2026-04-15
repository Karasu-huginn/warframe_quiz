use rusqlite::{params, Connection};
use crate::game::question_types::*;
use super::shuffle_answers;

pub fn generate(
    conn: &Connection,
    question_id: u64,
    time_limit: Option<u32>,
) -> Result<(Question, StoredQuestion), String> {
    let (boss_id, boss_name, correct_faction): (i64, String, String) = conn.query_row(
        "SELECT id, name, faction FROM bosses ORDER BY RANDOM() LIMIT 1",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|e| format!("no boss found: {e}"))?;

    // Distinct factions from other bosses
    let mut wrongs: Vec<String> = conn.prepare(
        "SELECT DISTINCT faction FROM bosses WHERE faction != ?1 ORDER BY RANDOM() LIMIT 3"
    ).map_err(|e| e.to_string())?
    .query_map(params![correct_faction], |row| row.get(0))
    .map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    // Fallback: try syndicates table for additional faction names
    if wrongs.len() < 3 {
        let needed = (3 - wrongs.len()) as i64;
        let more: Vec<String> = conn.prepare(
            "SELECT DISTINCT name FROM syndicates WHERE name != ?1 ORDER BY RANDOM() LIMIT ?2"
        ).map_err(|e| e.to_string())?
        .query_map(params![correct_faction, needed], |row| row.get(0))
        .map_err(|e| e.to_string())?.filter_map(|r| r.ok())
        .filter(|f| !wrongs.contains(f)).collect();
        wrongs.extend(more);
    }

    // Last fallback: hardcoded common Warframe factions
    if wrongs.len() < 3 {
        let fallbacks = ["Grineer", "Corpus", "Infested", "Orokin", "Sentient", "Narmer"];
        for f in &fallbacks {
            if wrongs.len() >= 3 { break; }
            let s = f.to_string();
            if s != correct_faction && !wrongs.contains(&s) {
                wrongs.push(s);
            }
        }
    }

    if wrongs.len() < 3 { return Err("not enough factions for wrong answers".to_string()); }

    let wrongs = wrongs.into_iter().take(3).collect();
    let (answers, correct_index) = shuffle_answers(correct_faction, wrongs);

    Ok((
        Question {
            question_id, question_type: "BossFaction".to_string(),
            question_text: "À quelle faction appartient ce boss ?".to_string(),
            clue: Clue::Text(boss_name), answers, time_limit,
        },
        StoredQuestion {
            question_id, question_type: "BossFaction".to_string(),
            correct_answer_index: correct_index,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn setup(conn: &rusqlite::Connection) {
        for (name, faction) in &[
            ("Vor", "Grineer"),
            ("Ambulas", "Corpus"),
            ("Phorid", "Infested"),
            ("Lephantis", "Infested"),
        ] {
            conn.execute(
                "INSERT INTO bosses (name, faction) VALUES (?1, ?2)",
                params![name, faction],
            ).unwrap();
        }
    }

    #[test]
    fn test_generate() {
        let conn = test_db();
        setup(&conn);
        let (q, s) = generate(&conn, 1, None).unwrap();
        assert_eq!(q.question_type, "BossFaction");
        assert_eq!(q.answers.len(), 4);
        assert!(s.correct_answer_index < 4);
        if let Clue::Text(boss_name) = &q.clue {
            assert!(!boss_name.is_empty());
        } else { panic!("expected Text clue"); }
        assert!(!q.answers[s.correct_answer_index].text.is_empty());
    }
}
