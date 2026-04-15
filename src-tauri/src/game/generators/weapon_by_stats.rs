use rusqlite::{params, Connection};
use crate::game::question_types::*;
use super::shuffle_answers;

pub fn generate(
    conn: &Connection,
    question_id: u64,
    time_limit: Option<u32>,
) -> Result<(Question, StoredQuestion), String> {
    let (wp_id, wp_name, wp_type, crit_chance, crit_mult, status_chance, fire_rate): (i64, String, String, f64, f64, f64, f64) = conn.query_row(
        "SELECT id, name, type, crit_chance, crit_multiplier, status_chance, fire_rate FROM weapons
         WHERE crit_chance IS NOT NULL
         ORDER BY RANDOM() LIMIT 1",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
    ).map_err(|e| format!("no weapon with stats: {e}"))?;

    let stats = vec![
        ("Chance critique".to_string(), format!("{:.0}%", crit_chance * 100.0)),
        ("Multiplicateur critique".to_string(), format!("x{:.1}", crit_mult)),
        ("Chance de statut".to_string(), format!("{:.0}%", status_chance * 100.0)),
        ("Cadence de tir".to_string(), format!("{:.2}", fire_rate)),
    ];

    // Type-matched wrong answers with fallback
    let mut wrongs: Vec<String> = conn.prepare(
        "SELECT name FROM weapons WHERE type = ?1 AND id != ?2 ORDER BY RANDOM() LIMIT 3"
    ).map_err(|e| e.to_string())?
    .query_map(params![wp_type, wp_id], |row| row.get(0))
    .map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    if wrongs.len() < 3 {
        let more: Vec<String> = conn.prepare(
            "SELECT name FROM weapons WHERE id != ?1 ORDER BY RANDOM() LIMIT ?2"
        ).map_err(|e| e.to_string())?
        .query_map(params![wp_id, (3 - wrongs.len()) as i64], |row| row.get(0))
        .map_err(|e| e.to_string())?.filter_map(|r| r.ok())
        .filter(|n| !wrongs.contains(n)).collect();
        wrongs.extend(more);
    }

    if wrongs.len() < 3 { return Err("not enough weapons".to_string()); }

    let (answers, correct_index) = shuffle_answers(wp_name, wrongs);

    Ok((
        Question {
            question_id, question_type: "WeaponByStats".to_string(),
            question_text: "Quelle arme a ces statistiques ?".to_string(),
            clue: Clue::StatBlock { stats }, answers, time_limit,
        },
        StoredQuestion {
            question_id, question_type: "WeaponByStats".to_string(),
            correct_answer_index: correct_index,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn setup(conn: &rusqlite::Connection) {
        for (name, t) in &[
            ("Braton","Primary"),
            ("Paris","Primary"),
            ("Boltor","Primary"),
            ("Soma","Primary"),
        ] {
            conn.execute(
                "INSERT INTO weapons (name, type, crit_chance, crit_multiplier, status_chance, fire_rate)
                 VALUES (?1, ?2, 0.28, 2.0, 0.10, 8.75)",
                params![name, t],
            ).unwrap();
        }
    }

    #[test]
    fn test_generate() {
        let conn = test_db();
        setup(&conn);
        let (q, s) = generate(&conn, 1, None).unwrap();
        assert_eq!(q.question_type, "WeaponByStats");
        assert_eq!(q.answers.len(), 4);
        assert!(s.correct_answer_index < 4);
        if let Clue::StatBlock { stats } = &q.clue {
            assert_eq!(stats.len(), 4);
            assert_eq!(stats[0].0, "Chance critique");
            assert_eq!(stats[0].1, "28%");
        } else { panic!("expected StatBlock clue"); }
    }
}
