//! Commandes IPC appelées par le frontend (`invoke`). Fines : la logique vit dans `palmanager-core`.

use crate::state::AppState;
use palmanager_core::{alerts, backup::{self, BackupInfo}, ini::{self, Options}, monitor::Snapshot, rest::RestClient, settings::AppSettings, steamcmd, Error, Result};
use std::{sync::atomic::Ordering, time::Duration};
use tauri::State;

type S<'a> = State<'a, AppState>;

#[tauri::command]
pub async fn get_settings(st: S<'_>) -> Result<AppSettings> { Ok(st.settings.read().await.clone()) }

#[tauri::command]
pub async fn save_settings(st: S<'_>, settings: AppSettings) -> Result<()> {
    settings.save(&st.settings_path)?;
    *st.settings.write().await = settings;
    Ok(())
}

#[tauri::command]
pub async fn server_start(st: S<'_>) -> Result<()> {
    st.expected_stop.store(false, Ordering::SeqCst);
    let s = st.settings.read().await.clone();
    st.server.start(&s).await
}

#[tauri::command]
pub async fn server_stop(st: S<'_>) -> Result<()> {
    st.expected_stop.store(true, Ordering::SeqCst);
    let s = st.settings.read().await.clone();
    st.server.stop(&s, Duration::from_secs(60)).await
}

#[tauri::command]
pub async fn server_restart(st: S<'_>) -> Result<()> {
    let s = st.settings.read().await.clone();
    st.expected_stop.store(true, Ordering::SeqCst);
    st.server.stop(&s, Duration::from_secs(60)).await?;
    st.expected_stop.store(false, Ordering::SeqCst);
    st.server.start(&s).await
}

#[tauri::command]
pub async fn get_snapshot(st: S<'_>) -> Result<Snapshot> { Ok(st.last_snapshot.read().await.clone()) }

#[tauri::command]
pub async fn read_world_settings(st: S<'_>) -> Result<Options> {
    let path = st.settings.read().await.world_settings_path();
    ini::parse(&std::fs::read_to_string(path)?)
}

#[tauri::command]
pub async fn write_world_settings(st: S<'_>, options: Options) -> Result<()> {
    let path = st.settings.read().await.world_settings_path();
    if path.exists() { std::fs::copy(&path, path.with_extension("ini.bak"))?; }
    std::fs::write(path, ini::serialize(&options))?;
    Ok(())
}

#[tauri::command]
pub async fn backup_now(st: S<'_>) -> Result<BackupInfo> {
    let s = st.settings.read().await.clone();
    if let Ok(api) = RestClient::new(&s.rest) { let _ = api.save().await; } // flush disque avant copie
    let info = backup::create(&s.save_dir(), &s.backup.destination)?;
    backup::rotate(&s.backup.destination, s.backup.retention)?;
    Ok(info)
}

#[tauri::command]
pub async fn list_backups(st: S<'_>) -> Result<Vec<BackupInfo>> {
    backup::list(&st.settings.read().await.backup.destination)
}

#[tauri::command]
pub async fn restore_backup(st: S<'_>, path: String) -> Result<()> {
    if st.server.is_running().await { return Err(Error::Other("arrêtez le serveur avant de restaurer".into())); }
    let s = st.settings.read().await.clone();
    // On n'accepte que des archives situées dans le dossier de sauvegarde configuré.
    let p = std::path::PathBuf::from(&path).canonicalize()?;
    if !p.starts_with(s.backup.destination.canonicalize()?) { return Err(Error::Other("archive hors du dossier de sauvegarde".into())); }
    backup::restore(&p, &s.save_dir())
}

#[tauri::command]
pub async fn test_alert(st: S<'_>) -> Result<()> {
    let cfg = st.settings.read().await.alerts.clone();
    alerts::dispatch(&cfg, &alerts::Event::PlayerJoined("Test".into())).await
}

#[tauri::command]
pub async fn announce(st: S<'_>, message: String) -> Result<()> {
    RestClient::new(&st.settings.read().await.rest)?.announce(&message).await
}

#[tauri::command]
pub async fn kick_player(st: S<'_>, user_id: String) -> Result<()> {
    RestClient::new(&st.settings.read().await.rest)?.kick(&user_id, "Expulsé par l'administrateur").await
}

#[tauri::command]
pub async fn ban_player(st: S<'_>, user_id: String) -> Result<()> {
    RestClient::new(&st.settings.read().await.rest)?.ban(&user_id, "Banni par l'administrateur").await
}

#[tauri::command]
pub async fn unban_player(st: S<'_>, user_id: String) -> Result<()> {
    RestClient::new(&st.settings.read().await.rest)?.unban(&user_id).await
}

/// Arrête le serveur, met à jour via SteamCMD, puis relance s'il tournait. Renvoie la fin du log SteamCMD.
#[tauri::command]
pub async fn update_server(st: S<'_>) -> Result<String> {
    if st.maintenance.swap(true, Ordering::SeqCst) { return Err(Error::Other("une maintenance est déjà en cours".into())); }
    let s = st.settings.read().await.clone();
    let was_running = st.server.is_running().await;
    let res = async {
        if was_running {
            if let Ok(api) = RestClient::new(&s.rest) { let _ = api.save().await; }
            let _ = backup::create(&s.save_dir(), &s.backup.destination);
            st.expected_stop.store(true, Ordering::SeqCst);
            st.server.stop(&s, Duration::from_secs(60)).await?;
        }
        let out = steamcmd::update(&s.steamcmd_path, &s.server_dir).await;
        st.expected_stop.store(false, Ordering::SeqCst);
        if was_running { st.server.start(&s).await?; }
        out
    }.await;
    st.maintenance.store(false, Ordering::SeqCst);
    res
}
