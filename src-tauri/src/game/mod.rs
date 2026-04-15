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
            "UPDATE quiz_sessions SET score = ?1, total_questions = ?2 WHERE id = ?3",
            params![self.score, self.total, self.id],
        ).map_err(|e| e.to_string())?;
        Ok(self.stats())
    }
}
