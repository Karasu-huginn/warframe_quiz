pub mod question_types;
pub mod generators;

use std::sync::Mutex;
use rusqlite::{params, Connection};
use question_types::*;

pub struct GameState {
    pub session: Mutex<Option<QuizSession>>,
    next_question_id: Mutex<u64>,
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            session: Mutex::new(None),
            next_question_id: Mutex::new(1),
        }
    }

    pub fn next_id(&self) -> u64 {
        let mut id = self.next_question_id.lock().unwrap();
        let current = *id;
        *id += 1;
        current
    }
}

pub struct QuizSession {
    pub id: i64,
    pub score: u32,
    pub total: u32,
    pub current_streak: u32,
    pub best_streak: u32,
    pub timer_enabled: bool,
    pub timer_seconds: u32,
    pub current_question: Option<StoredQuestion>,
}

impl QuizSession {
    pub fn start(conn: &Connection, timer_enabled: bool, timer_seconds: u32) -> Result<Self, String> {
        conn.execute(
            "INSERT INTO quiz_sessions (started_at, mode, score, total_questions) VALUES (datetime('now'), 'mixed', 0, 0)",
            [],
        ).map_err(|e| e.to_string())?;
        let id = conn.last_insert_rowid();
        Ok(QuizSession {
            id, score: 0, total: 0, current_streak: 0, best_streak: 0,
            timer_enabled, timer_seconds, current_question: None,
        })
    }

    pub fn submit_answer(
        &mut self,
        conn: &Connection,
        answer_index: usize,
        elapsed_seconds: Option<f64>,
    ) -> Result<AnswerResult, String> {
        let stored = self.current_question.take().ok_or("no question pending")?;
        let timed_out = self.timer_enabled
            && elapsed_seconds.map(|e| e > self.timer_seconds as f64).unwrap_or(false);
        let is_correct = !timed_out && answer_index == stored.correct_answer_index;

        self.total += 1;
        if is_correct {
            self.score += 1;
            self.current_streak += 1;
            if self.current_streak > self.best_streak {
                self.best_streak = self.current_streak;
            }
        } else {
            self.current_streak = 0;
        }

        conn.execute(
            "INSERT INTO quiz_answers (session_id, category, correct_item_id, chosen_item_id, is_correct, answered_at) VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
            params![self.id, stored.question_type, stored.correct_answer_index as i64, answer_index as i64, is_correct],
        ).map_err(|e| e.to_string())?;

        Ok(AnswerResult {
            is_correct,
            correct_answer_index: stored.correct_answer_index,
            score: self.score,
            total: self.total,
            current_streak: self.current_streak,
            best_streak: self.best_streak,
        })
    }

    pub fn stats(&self) -> SessionStats {
        SessionStats {
            session_id: self.id,
            score: self.score,
            total: self.total,
            current_streak: self.current_streak,
            best_streak: self.best_streak,
        }
    }

    pub fn end(&self, conn: &Connection) -> Result<SessionStats, String> {
        conn.execute(
            "UPDATE quiz_sessions SET score = ?1, total_questions = ?2, best_streak = ?3 WHERE id = ?4",
            params![self.score, self.total, self.best_streak, self.id],
        ).map_err(|e| e.to_string())?;
        Ok(self.stats())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn setup_quiz_data(conn: &Connection) {
        for (name, wf_type) in &[("Excalibur","Warframe"),("Mag","Warframe"),("Volt","Warframe"),("Frost","Warframe")] {
            conn.execute("INSERT INTO warframes (name, type) VALUES (?1, ?2)", params![name, wf_type]).unwrap();
        }
        let id: i64 = conn.query_row("SELECT id FROM warframes WHERE name='Excalibur'", [], |r| r.get(0)).unwrap();
        for (name, slot) in &[("Slash Dash",1),("Radial Blind",2),("Radial Javelin",3),("Exalted Blade",4)] {
            conn.execute("INSERT INTO abilities (name, warframe_id, slot_index) VALUES (?1,?2,?3)", params![name, id, slot]).unwrap();
        }
    }

    #[test]
    fn test_start_and_end_session() {
        let conn = test_db();
        let session = QuizSession::start(&conn, false, 0).unwrap();
        assert!(session.id > 0);
        assert_eq!(session.score, 0);
        assert_eq!(session.total, 0);
        let stats = session.end(&conn).unwrap();
        assert_eq!(stats.score, 0);
        // Verify saved to DB
        let saved: i64 = conn.query_row("SELECT score FROM quiz_sessions WHERE id=?1", params![session.id], |r| r.get(0)).unwrap();
        assert_eq!(saved, 0);
    }

    #[test]
    fn test_submit_correct_answer() {
        let conn = test_db();
        let mut session = QuizSession::start(&conn, false, 0).unwrap();
        session.current_question = Some(StoredQuestion {
            question_id: 1, question_type: "Test".to_string(), correct_answer_index: 2,
        });
        let result = session.submit_answer(&conn, 2, None).unwrap();
        assert!(result.is_correct);
        assert_eq!(result.score, 1);
        assert_eq!(result.total, 1);
        assert_eq!(result.current_streak, 1);
    }

    #[test]
    fn test_submit_wrong_answer() {
        let conn = test_db();
        let mut session = QuizSession::start(&conn, false, 0).unwrap();
        session.current_question = Some(StoredQuestion {
            question_id: 1, question_type: "Test".to_string(), correct_answer_index: 2,
        });
        let result = session.submit_answer(&conn, 0, None).unwrap();
        assert!(!result.is_correct);
        assert_eq!(result.score, 0);
        assert_eq!(result.current_streak, 0);
        assert_eq!(result.correct_answer_index, 2);
    }

    #[test]
    fn test_streak_tracking() {
        let conn = test_db();
        let mut session = QuizSession::start(&conn, false, 0).unwrap();

        // 3 correct
        for _ in 0..3 {
            session.current_question = Some(StoredQuestion {
                question_id: 1, question_type: "Test".to_string(), correct_answer_index: 0,
            });
            session.submit_answer(&conn, 0, None).unwrap();
        }
        assert_eq!(session.current_streak, 3);
        assert_eq!(session.best_streak, 3);

        // 1 wrong — resets current, keeps best
        session.current_question = Some(StoredQuestion {
            question_id: 2, question_type: "Test".to_string(), correct_answer_index: 0,
        });
        session.submit_answer(&conn, 1, None).unwrap();
        assert_eq!(session.current_streak, 0);
        assert_eq!(session.best_streak, 3);

        // 1 more correct
        session.current_question = Some(StoredQuestion {
            question_id: 3, question_type: "Test".to_string(), correct_answer_index: 0,
        });
        session.submit_answer(&conn, 0, None).unwrap();
        assert_eq!(session.current_streak, 1);
        assert_eq!(session.best_streak, 3);
    }

    #[test]
    fn test_timer_enforcement() {
        let conn = test_db();
        let mut session = QuizSession::start(&conn, true, 15).unwrap();
        session.current_question = Some(StoredQuestion {
            question_id: 1, question_type: "Test".to_string(), correct_answer_index: 0,
        });
        // Correct answer but too slow
        let result = session.submit_answer(&conn, 0, Some(20.0)).unwrap();
        assert!(!result.is_correct);
    }

    #[test]
    fn test_submit_without_question_fails() {
        let conn = test_db();
        let mut session = QuizSession::start(&conn, false, 0).unwrap();
        assert!(session.submit_answer(&conn, 0, None).is_err());
    }

    #[test]
    fn test_full_cycle_with_generator() {
        let conn = test_db();
        setup_quiz_data(&conn);
        let game_state = GameState::new();
        let mut session = QuizSession::start(&conn, false, 0).unwrap();

        let qid = game_state.next_id();
        let (question, stored) = generators::generate_question(&conn, qid, None).unwrap();
        assert_eq!(question.answers.len(), 4);

        session.current_question = Some(stored);
        let result = session.submit_answer(&conn, question.answers[0].index, None).unwrap();
        assert_eq!(result.total, 1);

        let final_stats = session.end(&conn).unwrap();
        assert_eq!(final_stats.total, 1);
    }
}
