mod commands;
mod cubase;
mod db;
mod models;
mod seed;

use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
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
            commands::filter_projets,
            commands::get_dashboard_stats,
            commands::get_chart_data,
            commands::get_dettes_comparaison,
            commands::save_binary_file,
            commands::find_client_by_nom,
            commands::create_client_avec_projet,
            cubase::get_cubase_settings,
            cubase::save_cubase_settings,
            cubase::preview_cubase_session,
            cubase::create_cubase_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
