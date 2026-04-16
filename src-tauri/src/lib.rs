mod db;
mod fetcher;
mod game;
mod commands;

use db::connection::Database;
use db::schema;
use tauri::Manager;
use game::GameState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data = app.path().app_data_dir().expect("no app data dir");
            std::fs::create_dir_all(&app_data).expect("cannot create app data dir");
            let db_path = app_data.join("warframedle.db");
            let database = Database::new(&db_path).expect("cannot open database");
            {
                let conn = database.conn.lock().unwrap();
                schema::create_tables(&conn).expect("cannot create tables");
            }
            app.manage(database);
            app.manage(GameState::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_db_stats,
            commands::fetch_wiki_data,
            commands::start_quiz,
            commands::next_question,
            commands::submit_answer,
            commands::get_session_stats,
            commands::end_quiz,
            commands::get_overall_stats,
            commands::get_recent_sessions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
