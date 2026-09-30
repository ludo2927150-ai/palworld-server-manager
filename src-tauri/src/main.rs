#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;
mod supervisor;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let st = state::AppState::load(app.handle())?;
            app.manage(st);
            supervisor::spawn(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings, commands::save_settings,
            commands::server_start, commands::server_stop, commands::server_restart,
            commands::get_snapshot,
            commands::read_world_settings, commands::write_world_settings,
            commands::backup_now, commands::list_backups, commands::restore_backup,
            commands::test_alert,
            commands::announce, commands::kick_player, commands::ban_player, commands::unban_player,
            commands::update_server, commands::get_history, commands::get_sessions, commands::read_logs,
            commands::diagnose, commands::fix_rest,
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de Tauri");
}

use tauri::Manager;
