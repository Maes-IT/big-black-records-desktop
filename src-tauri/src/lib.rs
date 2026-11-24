mod apparence;
mod commands;
mod cubase;
mod db;
mod models;

use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let conn = match db::init_db(&app_data_dir) {
                Ok(conn) => conn,
                Err(msg) => {
                    app.dialog()
                        .message(format!("{msg}\n\nL'application va se fermer."))
                        .title("Big Black Records — erreur de base de données")
                        .kind(MessageDialogKind::Error)
                        .blocking_show();
                    std::process::exit(1);
                }
            };
            // Sauvegarde automatique du jour : un échec ne doit pas empêcher de travailler.
            if let Err(e) = db::auto_backup(&conn, &db::backup_dir(&app_data_dir)) {
                eprintln!("Sauvegarde automatique impossible : {e}");
            }
            app.manage(db::DbState(Mutex::new(conn)));
            app.manage(db::DataDir(app_data_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_clients,
            commands::get_client,
            commands::create_client,
            commands::update_client,
            commands::delete_client,
            commands::list_projets,
            commands::list_all_projets,
            commands::get_chart_years,
            commands::get_backup_info,
            commands::backup_db,
            commands::restore_db,
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
            apparence::get_apparence,
            apparence::save_apparence,
            apparence::set_fond_image,
            apparence::get_fond_image,
            apparence::clear_fond_image,
            cubase::get_cubase_settings,
            cubase::save_cubase_settings,
            cubase::preview_cubase_session,
            cubase::create_cubase_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
