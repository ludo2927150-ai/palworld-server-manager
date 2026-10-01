//! Commandes IPC appelées par le frontend (`invoke`). Fines : la logique vit dans `palmanager-core`.

use crate::state::AppState;
use palmanager_core::{alerts, backup::{self, BackupInfo}, history::{self, Sample, Session}, logs::{self, LogChunk}, setup::{self, Check}, ini::{self, Options}, monitor::Snapshot, rest::RestClient, settings::AppSettings, steamcmd, Error, Result};
use std::{sync::atomic::Ordering, time::Duration};
use tauri::State;

type S<'a> = State<'a, AppState>;

#[tauri::command]
pub async fn get_settings(st: S<'_>) -> Result<AppSettings> { Ok(st.settings.read().await.clone()) }

#[tauri::command]
pub async fn save_settings(app: tauri::AppHandle, st: S<'_>, mut settings: AppSettings) -> Result<()> {
    if settings.remote.enabled { settings.remote.ensure_token()?; }
    settings.save(&st.settings_path)?;
    *st.settings.write().await = settings;
    crate::remote::apply(&app).await;
    Ok(())
}

#[derive(serde::Serialize)]
pub struct RemoteUrl {
    pub label: String,
    pub url: String,
}

#[derive(serde::Serialize)]
pub struct RemoteInfo {
    pub running: bool,
    pub error: Option<String>,
    /// Adresses à ouvrir depuis le téléphone (le jeton est dans le fragment `#token=`, jamais envoyé au réseau).
    pub urls: Vec<RemoteUrl>,
    /// Mêmes adresses sans clé (`http://ip:port/`) : sert à composer le lien d'un invité avec sa propre clé.
    pub bases: Vec<RemoteUrl>,
    /// Adresse Tailscale détectée sur ce PC (accès hors de la maison) ; `false` = Tailscale absent ou déconnecté.
    pub tailscale_found: bool,
}

#[tauri::command]
pub async fn remote_info(st: S<'_>) -> Result<RemoteInfo> {
    let cfg = st.settings.read().await.remote.clone();
    let tailscale = tokio::task::spawn_blocking(palmanager_core::net::tailscale_ip).await.ok().flatten();
    let lan = palmanager_core::net::lan_ip();
    let (mut urls, mut bases) = (Vec::new(), Vec::new());
    if cfg.enabled {
        let mut add = |label: &str, ip: std::net::IpAddr| {
            let base = format!("http://{ip}:{}/", cfg.port);
            urls.push(RemoteUrl { label: label.into(), url: format!("{base}#token={}", cfg.token) });
            bases.push(RemoteUrl { label: label.into(), url: base });
        };
        if let Some(ip) = tailscale { add("Partout (Tailscale, 4G ou Wi-Fi)", ip); }
        if let Some(ip) = lan { add("Chez vous (même Wi-Fi)", ip); }
    }
    Ok(RemoteInfo { running: st.remote.lock().await.is_some(), error: st.remote_error.lock().await.clone(), urls, bases, tailscale_found: tailscale.is_some() })
}

/// Invalide l'ancien jeton (les téléphones déjà connectés devront se reconnecter).
#[tauri::command]
pub async fn regenerate_remote_token(app: tauri::AppHandle, st: S<'_>) -> Result<()> {
    let mut s = st.settings.read().await.clone();
    s.remote.token = palmanager_core::settings::RemoteSettings::generate_token()?;
    s.save(&st.settings_path)?;
    *st.settings.write().await = s;
    crate::remote::apply(&app).await;
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
pub async fn read_world_settings(st: S<'_>) -> Result<WorldSettings> {
    let (options, from_default) = st.settings.read().await.load_world_options()?;
    Ok(WorldSettings { options, from_default })
}

#[derive(serde::Serialize)]
pub struct WorldSettings {
    pub options: Options,
    /// `true` : PalWorldSettings.ini était vide, les valeurs viennent de DefaultPalWorldSettings.ini.
    pub from_default: bool,
}

#[tauri::command]
pub async fn write_world_settings(st: S<'_>, options: Options) -> Result<()> {
    let path = st.settings.read().await.world_settings_path();
    if let Some(dir) = path.parent() { std::fs::create_dir_all(dir)?; }
    if path.exists() { std::fs::copy(&path, path.with_extension("ini.bak"))?; }
    std::fs::write(path, ini::serialize(&options))?;
    Ok(())
}

#[tauri::command]
pub async fn backup_now(st: S<'_>) -> Result<BackupInfo> {
    let s = st.settings.read().await.clone();
    if let Ok(api) = RestClient::new(&s.rest) { let _ = api.save().await; } // flush disque avant copie
    tokio::task::spawn_blocking(move || backup::backup_all(&s, Some("manuel"))).await.map_err(|e| Error::Other(e.to_string()))?
}

#[tauri::command]
pub async fn list_backups(st: S<'_>) -> Result<Vec<BackupInfo>> {
    backup::list(&st.settings.read().await.backup.destination)
}

/// Revenir à une sauvegarde en un clic : arrête le serveur s'il tourne (ce qui sauvegarde l'état actuel),
/// restaure l'archive choisie, puis relance le serveur s'il tournait. L'état précédent reste récupérable
/// (archive `-arret` ou `-avant-restauration`, et dossier `SaveGames.bak`).
#[tauri::command]
pub async fn restore_backup(st: S<'_>, path: String) -> Result<()> {
    let s = st.settings.read().await.clone();
    // On n'accepte que des archives situées dans le dossier de sauvegarde configuré.
    let p = std::path::PathBuf::from(&path).canonicalize()?;
    if !p.starts_with(s.backup.destination.canonicalize()?) { return Err(Error::Other("archive hors du dossier de sauvegarde".into())); }
    if st.maintenance.swap(true, Ordering::SeqCst) { return Err(Error::Other("une maintenance est déjà en cours".into())); }
    let was_running = st.server.is_running().await;
    let tmp = std::env::temp_dir().join(format!("palworld-restore-{}.zip", chrono::Utc::now().timestamp()));
    let res: Result<()> = async {
        // Copie de sûreté : la rotation déclenchée par la sauvegarde d'arrêt pourrait supprimer l'archive choisie.
        std::fs::copy(&p, &tmp)?;
        if was_running {
            st.expected_stop.store(true, Ordering::SeqCst);
            st.server.stop(&s, Duration::from_secs(60)).await?; // sauvegarde l'état actuel si backup.on_stop
        }
        if !(was_running && s.backup.on_stop) {
            let s2 = s.clone();
            let _ = tokio::task::spawn_blocking(move || backup::backup_all(&s2, Some("avant-restauration"))).await;
        }
        let (t2, dir) = (tmp.clone(), s.save_dir());
        tokio::task::spawn_blocking(move || backup::restore(&t2, &dir)).await.map_err(|e| Error::Other(e.to_string()))??;
        Ok(())
    }.await;
    let _ = std::fs::remove_file(&tmp);
    if was_running {
        st.expected_stop.store(false, Ordering::SeqCst);
        if res.is_ok() { let _ = st.server.start(&s).await; }
    }
    st.maintenance.store(false, Ordering::SeqCst);
    res
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
            st.expected_stop.store(true, Ordering::SeqCst);
            st.server.stop(&s, Duration::from_secs(60)).await?;
        }
        let out = steamcmd::update(&s.steamcmd_exe(), &s.server_dir).await;
        st.expected_stop.store(false, Ordering::SeqCst);
        if was_running { st.server.start(&s).await?; }
        out
    }.await;
    st.maintenance.store(false, Ordering::SeqCst);
    res
}

#[tauri::command]
pub async fn get_history(st: S<'_>, hours: i64) -> Result<Vec<Sample>> {
    let since = chrono::Utc::now().timestamp() - hours.clamp(1, 168) * 3600;
    Ok(st.history.samples_since(since, 600))
}

#[tauri::command]
pub async fn get_sessions(st: S<'_>, days: i64) -> Result<Vec<Session>> {
    let since = chrono::Utc::now().timestamp() - days.clamp(1, 30) * 86_400;
    let mut v = history::sessions(&st.history.events_since(since));
    v.reverse(); // plus récentes d'abord
    Ok(v)
}

#[tauri::command]
pub async fn read_logs(st: S<'_>, offset: Option<u64>) -> Result<LogChunk> {
    let path = st.settings.read().await.server_dir.join("Pal/Saved/Logs/Pal.log");
    logs::read_from(&path, offset)
}

#[tauri::command]
pub async fn diagnose(st: S<'_>) -> Result<Vec<Check>> {
    let s = st.settings.read().await.clone();
    Ok(setup::diagnose(&s).await)
}

/// Active l'API REST dans le .ini et aligne les paramètres de l'app. Le serveur doit être redémarré ensuite.
#[tauri::command]
pub async fn fix_rest(st: S<'_>, admin_password: String) -> Result<()> {
    let s = st.settings.read().await.clone();
    let fixed = setup::fix_rest(&s, &admin_password)?;
    fixed.save(&st.settings_path)?;
    *st.settings.write().await = fixed;
    Ok(())
}

#[tauri::command]
pub async fn get_autostart() -> Result<bool> { palmanager_core::autostart::is_enabled() }

#[tauri::command]
pub async fn set_autostart(enabled: bool) -> Result<()> {
    // La version de développement s'ouvre dans une console noire et sans interface (pas de serveur Vite au démarrage de Windows).
    // On autorise seulement la désactivation : installer l'application (npm run tauri build) avant d'activer.
    if enabled && cfg!(debug_assertions) {
        return Err(Error::Other("indisponible en mode développement : installez l'application (npm run tauri build) puis activez cette option depuis la version installée".into()));
    }
    let exe = std::env::current_exe()?;
    palmanager_core::autostart::set_enabled(&exe, enabled)
}

#[derive(serde::Serialize)]
pub struct NetworkInfo {
    /// IP du PC sur le réseau local (pour les joueurs connectés au même Wi-Fi/box).
    pub lan_ip: Option<String>,
}

#[tauri::command]
pub async fn network_info() -> Result<NetworkInfo> {
    Ok(NetworkInfo { lan_ip: palmanager_core::net::lan_ip().map(|ip| ip.to_string()) })
}

/// IP publique : appelle un service externe (api.ipify.org), uniquement quand l'utilisateur clique.
#[tauri::command]
pub async fn public_ip() -> Result<String> { palmanager_core::net::public_ip().await }

#[tauri::command]
pub async fn system_info() -> Result<palmanager_core::monitor::SystemInfo> { Ok(palmanager_core::monitor::system_info()) }

/// Applique tout de suite priorité et cœurs aux processus du serveur en cours ; renvoie leur nombre.
#[tauri::command]
pub async fn apply_performance(st: S<'_>) -> Result<usize> {
    let p = st.settings.read().await.performance.clone();
    let cores = palmanager_core::monitor::system_info().cpu_cores;
    tokio::task::spawn_blocking(move || palmanager_core::perf::apply(&p, cores)).await.map_err(|e| Error::Other(e.to_string()))?
}

/// Crée une invitation : clé propre, permissions choisies, durée de validité optionnelle. Effet immédiat.
#[tauri::command]
pub async fn create_guest(app: tauri::AppHandle, st: S<'_>, name: String, perms: Vec<palmanager_core::settings::Perm>, hours: Option<u32>) -> Result<palmanager_core::settings::Guest> {
    let name = name.trim().to_string();
    if name.is_empty() || name.chars().count() > 40 { return Err(Error::Other("nom invalide (1 à 40 caractères)".into())); }
    let mut perms = perms;
    perms.sort_by_key(|p| palmanager_core::settings::Perm::ALL.iter().position(|x| x == p));
    perms.dedup();
    if perms.is_empty() { return Err(Error::Other("choisissez au moins une permission".into())); }
    let now = chrono::Utc::now().timestamp();
    let guest = palmanager_core::settings::Guest {
        id: palmanager_core::settings::RemoteSettings::generate_token()?[..12].to_string(),
        name,
        token: palmanager_core::settings::RemoteSettings::generate_token()?,
        perms,
        expires_at: hours.filter(|h| *h > 0).map(|h| now + i64::from(h) * 3600),
        created_at: now,
    };
    let mut s = st.settings.read().await.clone();
    s.remote.guests.push(guest.clone());
    s.save(&st.settings_path)?;
    *st.settings.write().await = s;
    crate::remote::apply(&app).await;
    Ok(guest)
}

/// Révoque une invitation : sa clé cesse de fonctionner immédiatement.
#[tauri::command]
pub async fn revoke_guest(app: tauri::AppHandle, st: S<'_>, id: String) -> Result<()> {
    let mut s = st.settings.read().await.clone();
    s.remote.guests.retain(|g| g.id != id);
    s.save(&st.settings_path)?;
    *st.settings.write().await = s;
    crate::remote::apply(&app).await;
    Ok(())
}
