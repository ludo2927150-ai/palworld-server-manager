#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod remote;
mod state;
mod supervisor;
mod tray;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .on_window_event(|window, event| {
            // « Fermer » réduit dans la zone de notification si l'option est activée.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let to_tray = window.app_handle().try_state::<state::AppState>()
                    .and_then(|st| st.settings.try_read().ok().map(|s| s.close_to_tray)).unwrap_or(false);
                if to_tray { let _ = window.hide(); api.prevent_close(); }
            }
        })
        .setup(|app| {
            let st = state::AppState::load(app.handle())?;
            app.manage(st);
            supervisor::spawn(app.handle().clone());
            tray::setup(app)?;
            // Lancé par Windows au démarrage (--minimized) : on démarre dans la zone de notification.
            if std::env::args().any(|a| a == "--minimized") {
                if let Some(w) = app.get_webview_window("main") { let _ = w.hide(); }
            }
            let h = app.handle().clone();
            tauri::async_runtime::spawn(async move { remote::apply(&h).await });
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
            commands::update_server, commands::get_autostart, commands::set_autostart, commands::get_history, commands::get_sessions, commands::read_logs,
            commands::diagnose, commands::fix_rest, commands::network_info, commands::system_info, commands::apply_performance, commands::public_ip,
            commands::analyze_log, commands::players_known, commands::players_bans, commands::mods_state, commands::mods_set_global, commands::mods_set_root, commands::mods_set_active, commands::mods_add, commands::mods_remove, commands::remote_info, commands::regenerate_remote_token, commands::create_guest, commands::revoke_guest,
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de Tauri");
}

use tauri::Manager;
