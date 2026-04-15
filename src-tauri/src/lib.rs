mod db;
mod fetcher;
mod game;
mod commands;

use db::connection::Database;
use db::schema;
use tauri::Manager;

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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_db_stats,
            commands::fetch_wiki_data
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
