mod commands;
mod db;
mod models;
mod seed;

use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let conn = db::init_db(&app_data_dir);
            app.manage(db::DbState(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_clients,
            commands::get_client,
            commands::create_client,
            commands::update_client,
            commands::delete_client,
            commands::list_projets,
            commands::create_projet,
            commands::update_projet,
            commands::delete_projet,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
