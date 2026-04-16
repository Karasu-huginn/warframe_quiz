use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use crate::db::connection::Database;
use crate::db::queries::{warframes, abilities, weapons, mods};
use crate::fetcher::coordinator::{self, FetchReport, FetchProgress};
use crate::db::schema;
use rusqlite::Connection;
use std::path::Path;
use crate::game::{GameState, QuizSession};
use crate::game::question_types::*;
use crate::game::generators;

#[derive(Serialize)]
pub struct DbStats {
    pub warframe_count: i64,
    pub ability_count: i64,
    pub weapon_count: i64,
    pub mod_count: i64,
}

#[tauri::command]
pub fn fetch_wiki_data(db: State<'_, Database>, app: AppHandle) -> Result<FetchReport, String> {
    let conn = Connection::open(&db.path).map_err(|e| format!("DB open error: {e}"))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| format!("PRAGMA error: {e}"))?;
    schema::create_tables(&conn).map_err(|e| format!("schema error: {e}"))?;

    let assets_dir = db.path.parent().unwrap_or(Path::new(".")).join("assets");

    let report = coordinator::fetch_all(&conn, &assets_dir, &|progress: FetchProgress| {
        let _ = app.emit("fetch_progress", &progress);
    });

    Ok(report)
}

#[tauri::command]
pub fn get_db_stats(db: State<'_, Database>) -> Result<DbStats, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(DbStats {
        warframe_count: warframes::get_warframe_count(&conn).map_err(|e| e.to_string())?,
        ability_count: abilities::get_ability_count(&conn).map_err(|e| e.to_string())?,
        weapon_count: weapons::get_weapon_count(&conn).map_err(|e| e.to_string())?,
        mod_count: mods::get_mod_count(&conn).map_err(|e| e.to_string())?,
    })
}

#[tauri::command]
pub fn start_quiz(
    db: State<'_, Database>,
    game: State<'_, GameState>,
    timer_enabled: bool,
    timer_seconds: u32,
) -> Result<SessionStats, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut session_lock = game.session.lock().map_err(|e| e.to_string())?;
    if let Some(prev) = session_lock.take() {
        let _ = prev.end(&conn);
    }
    let session = QuizSession::start(&conn, timer_enabled, timer_seconds)?;
    let stats = session.stats();
    *session_lock = Some(session);
    Ok(stats)
}

#[tauri::command]
pub fn next_question(
    db: State<'_, Database>,
    game: State<'_, GameState>,
) -> Result<Question, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let qid = game.next_id();
    let mut session_lock = game.session.lock().map_err(|e| e.to_string())?;
    let session = session_lock.as_mut().ok_or("no active session")?;
    let time_limit = if session.timer_enabled { Some(session.timer_seconds) } else { None };
    let (question, stored) = generators::generate_question(&conn, qid, time_limit)?;
    session.current_question = Some(stored);
    Ok(question)
}

#[tauri::command]
pub fn submit_answer(
    db: State<'_, Database>,
    game: State<'_, GameState>,
    answer_index: usize,
    elapsed_seconds: Option<f64>,
) -> Result<AnswerResult, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut session_lock = game.session.lock().map_err(|e| e.to_string())?;
    let session = session_lock.as_mut().ok_or("no active session")?;
    session.submit_answer(&conn, answer_index, elapsed_seconds)
}

#[tauri::command]
pub fn get_session_stats(
    game: State<'_, GameState>,
) -> Result<SessionStats, String> {
    let session_lock = game.session.lock().map_err(|e| e.to_string())?;
    let session = session_lock.as_ref().ok_or("no active session")?;
    Ok(session.stats())
}

#[tauri::command]
pub fn end_quiz(
    db: State<'_, Database>,
    game: State<'_, GameState>,
) -> Result<SessionStats, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut session_lock = game.session.lock().map_err(|e| e.to_string())?;
    let session = session_lock.take().ok_or("no active session")?;
    session.end(&conn)
}

#[derive(Serialize)]
pub struct OverallStats {
    pub total_games: i64,
    pub best_streak: i64,
    pub total_correct: i64,
    pub total_answered: i64,
}

#[derive(Serialize)]
pub struct RecentSession {
    pub id: i64,
    pub started_at: String,
    pub score: i64,
    pub total_questions: i64,
    pub best_streak: i64,
}

#[tauri::command]
pub fn get_overall_stats(db: State<'_, Database>) -> Result<OverallStats, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT COUNT(*), COALESCE(MAX(best_streak), 0), COALESCE(SUM(score), 0), COALESCE(SUM(total_questions), 0) FROM quiz_sessions",
        [],
        |row| Ok(OverallStats {
            total_games: row.get(0)?,
            best_streak: row.get(1)?,
            total_correct: row.get(2)?,
            total_answered: row.get(3)?,
        }),
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_recent_sessions(db: State<'_, Database>, limit: i64) -> Result<Vec<RecentSession>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, started_at, score, total_questions, best_streak FROM quiz_sessions ORDER BY id DESC LIMIT ?1"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(rusqlite::params![limit], |row| Ok(RecentSession {
        id: row.get(0)?,
        started_at: row.get(1)?,
        score: row.get(2)?,
        total_questions: row.get(3)?,
        best_streak: row.get(4)?,
    })).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_asset_base64(db: State<'_, Database>, relative_path: String) -> Result<String, String> {
    use base64::Engine;
    let full_path = db.path.parent().unwrap_or(Path::new(".")).join("assets").join(&relative_path);
    let bytes = std::fs::read(&full_path).map_err(|e| format!("cannot read {}: {e}", full_path.display()))?;
    let ext = full_path.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let mime = match ext {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "image/png",
    };
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{mime};base64,{b64}"))
}
