//! Commandes IPC appelées par le frontend (`invoke`). Fines : la logique vit dans `palmanager-core`.

use crate::state::AppState;
use palmanager_core::{mods, alerts, backup::{self, BackupInfo}, history::{self, Sample, Session}, logs::{self, LogChunk}, setup::{self, Check}, ini::{self, Options}, monitor::Snapshot, rest::RestClient, settings::AppSettings, steamcmd, Error, Result};
use std::{sync::atomic::Ordering, time::Duration};
use tauri::State;

type S<'a> = State<'a, AppState>;

#[tauri::command]
pub async fn get_settings(st: S<'_>) -> Result<AppSettings> { Ok(st.settings.read().await.clone()) }

#[tauri::command]
pub async fn save_settings(app: tauri::AppHandle, st: S<'_>, mut settings: AppSettings) -> Result<()> {
    settings.keep_backend_owned(&*st.settings.read().await);
    settings.sanitize();
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

pub async fn server_start_inner(st: &AppState) -> Result<()> {
    st.expected_stop.store(false, Ordering::SeqCst);
    let s = st.settings.read().await.clone();
    st.server.start(&s).await
}

pub async fn server_stop_inner(st: &AppState) -> Result<()> {
    st.expected_stop.store(true, Ordering::SeqCst);
    let s = st.settings.read().await.clone();
    st.server.stop(&s, Duration::from_secs(60)).await
}

pub async fn server_restart_inner(st: &AppState) -> Result<()> {
    let s = st.settings.read().await.clone();
    st.expected_stop.store(true, Ordering::SeqCst);
    st.server.stop(&s, Duration::from_secs(60)).await?;
    st.expected_stop.store(false, Ordering::SeqCst);
    st.server.start(&s).await
}

#[tauri::command]
pub async fn server_start(st: S<'_>) -> Result<()> { st.audit.record("app", "serveur : démarrage", ""); server_start_inner(&st).await }

#[tauri::command]
pub async fn server_stop(st: S<'_>) -> Result<()> { st.audit.record("app", "serveur : arrêt", ""); server_stop_inner(&st).await }

#[tauri::command]
pub async fn server_restart(st: S<'_>) -> Result<()> { st.audit.record("app", "serveur : redémarrage", ""); server_restart_inner(&st).await }

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
    ini::validate(&options)?;
    st.audit.record("app", "configuration du monde modifiée", "");
    let path = st.settings.read().await.world_settings_path();
    if let Some(dir) = path.parent() { std::fs::create_dir_all(dir)?; }
    if path.exists() { std::fs::copy(&path, path.with_extension("ini.bak"))?; }
    std::fs::write(path, ini::serialize(&options))?;
    Ok(())
}

pub async fn backup_now_inner(st: &AppState, label: String) -> Result<BackupInfo> {
    let s = st.settings.read().await.clone();
    if let Ok(api) = RestClient::new(&s.rest) { let _ = api.save().await; } // flush disque avant copie
    tokio::task::spawn_blocking(move || backup::backup_all(&s, Some(&label))).await.map_err(|e| Error::Other(e.to_string()))?
}

#[tauri::command]
pub async fn backup_now(st: S<'_>) -> Result<BackupInfo> { st.audit.record("app", "sauvegarde manuelle", ""); backup_now_inner(&st, "manuel".into()).await }

/// Sauvegarde « gardée » (avant un événement important) : jamais supprimée par la rotation tant qu'elle n'est pas déprotégée.
#[tauri::command]
pub async fn backup_now_protected(st: S<'_>, name: Option<String>) -> Result<BackupInfo> {
    let clean: String = name.unwrap_or_default().chars().filter(|c| c.is_ascii_alphanumeric()).take(20).collect();
    let label = format!("garde{}{}", if clean.is_empty() { "" } else { "-" }, clean);
    st.audit.record("app", "sauvegarde protégée", &clean);
    backup_now_inner(&st, label).await
}

/// Protège ou déprotège une sauvegarde existante (elle est renommée).
#[tauri::command]
pub async fn backup_set_protected(st: S<'_>, path: String, on: bool) -> Result<()> {
    let dest = st.settings.read().await.backup.destination.canonicalize()?;
    let p = std::path::PathBuf::from(&path).canonicalize()?;
    if !p.starts_with(&dest) { return Err(Error::Other("archive hors du dossier de sauvegarde".into())); }
    backup::set_protected(&p, on)?;
    st.audit.record("app", if on { "sauvegarde protégée" } else { "sauvegarde déprotégée" }, &path);
    Ok(())
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
    st.audit.record("app", "restauration du monde", &path);
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
        // Archive vérifiée AVANT d'arrêter le serveur ou de toucher au monde actuel.
        { let t = tmp.clone(); tokio::task::spawn_blocking(move || backup::verify(&t)).await.map_err(|e| Error::Other(e.to_string()))??; }
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
pub async fn test_alert(app: tauri::AppHandle, st: S<'_>) -> Result<()> {
    use tauri_plugin_notification::NotificationExt;
    let cfg = st.settings.read().await.alerts.clone();
    if cfg.desktop {
        app.notification().builder().title("Palworld Server Manager").body("Notification de test").show().map_err(|e| Error::Other(e.to_string()))?;
    }
    alerts::dispatch(&cfg, &alerts::Event::PlayerJoined("Test".into())).await
}

pub async fn announce_inner(st: &AppState, message: &str) -> Result<()> {
    RestClient::new(&st.settings.read().await.rest)?.announce(message).await
}

pub async fn kick_inner(st: &AppState, user_id: &str) -> Result<()> {
    RestClient::new(&st.settings.read().await.rest)?.kick(user_id, "Expulsé par l'administrateur").await
}

#[tauri::command]
pub async fn announce(st: S<'_>, message: String) -> Result<()> { st.audit.record("app", "annonce", &message); announce_inner(&st, &message).await }

#[tauri::command]
pub async fn kick_player(st: S<'_>, user_id: String) -> Result<()> { st.audit.record("app", "expulsion", &user_id); kick_inner(&st, &user_id).await }

#[tauri::command]
pub async fn ban_player(st: S<'_>, user_id: String, name: Option<String>, reason: Option<String>) -> Result<()> {
    st.audit.record("app", "bannissement", &format!("{user_id} {}", reason.clone().unwrap_or_default()));
    let msg = reason.clone().filter(|r| !r.trim().is_empty()).unwrap_or_else(|| "Banni par l'administrateur".into());
    RestClient::new(&st.settings.read().await.rest)?.ban(&user_id, &msg).await?;
    st.bans.lock().await.add(palmanager_core::players::BanEntry {
        user_id, name: name.unwrap_or_default(), banned_at: chrono::Utc::now().timestamp(), reason: reason.filter(|r| !r.trim().is_empty()),
    })
}

#[tauri::command]
pub async fn unban_player(st: S<'_>, user_id: String) -> Result<()> {
    st.audit.record("app", "débannissement", &user_id);
    RestClient::new(&st.settings.read().await.rest)?.unban(&user_id).await?;
    st.bans.lock().await.remove(&user_id)
}

#[derive(serde::Serialize)]
pub struct KnownPlayerView {
    #[serde(flatten)]
    pub player: palmanager_core::players::KnownPlayer,
    pub online: bool,
    pub banned: bool,
    pub allowed: bool,
}

/// Tous les joueurs déjà vus, avec leur état actuel (en ligne, banni, autorisé).
#[tauri::command]
pub async fn players_known(st: S<'_>) -> Result<Vec<KnownPlayerView>> {
    let allowed: Vec<String> = st.settings.read().await.access.allowed.iter().map(|a| a.user_id.clone()).collect();
    let banned: Vec<String> = st.bans.lock().await.list().into_iter().map(|b| b.user_id).collect();
    let book = st.players.lock().await;
    Ok(book.list().into_iter().map(|p| KnownPlayerView {
        online: book.is_online(&p.user_id), banned: banned.contains(&p.user_id), allowed: allowed.contains(&p.user_id), player: p,
    }).collect())
}

#[tauri::command]
pub async fn players_bans(st: S<'_>) -> Result<Vec<palmanager_core::players::BanEntry>> { Ok(st.bans.lock().await.list()) }

/// Arrête le serveur, met à jour via SteamCMD, puis relance s'il tournait. Renvoie la fin du log SteamCMD.
#[tauri::command]
pub async fn update_server(st: S<'_>) -> Result<String> {
    st.audit.record("app", "mise à jour du serveur", "");
    let s = st.settings.read().await.clone();
    // Sauvegarde, mise à jour, redémarrage, vérification et retour arrière automatique si le serveur ne repart pas.
    Ok(crate::automation::guarded_cycle(&st, &s, "maj", true, &[], None).await)
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
    let s = st.settings.read().await;
    logs::read_located(&s.server_dir, s.log_file.as_deref(), st.server.console_log_path(), offset)
}

#[tauri::command]
pub fn detect_setup() -> (Vec<std::path::PathBuf>, Vec<std::path::PathBuf>) {
    (setup::detect_server_dirs(), setup::detect_steamcmd())
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

#[derive(serde::Serialize)]
pub struct ModsState {
    pub settings_path: String,
    pub global_enable: bool,
    pub workshop_root: Option<String>,
    pub root_exists: bool,
    /// Dossiers Workshop trouvés sur ce PC (serveur, client Steam).
    pub candidates: Vec<String>,
    /// Dossier utilisé pour les téléchargements automatiques (dans le dossier du serveur).
    pub download_root: String,
    pub mods: Vec<mods::ModInfo>,
}

fn mods_state_for(s: &AppSettings) -> ModsState {
    let ms = mods::load_settings(&s.server_dir);
    let candidates: Vec<std::path::PathBuf> = mods::candidate_roots(&s.server_dir);
    let root = ms.workshop_root.clone().map(std::path::PathBuf::from).or_else(|| candidates.first().cloned());
    ModsState {
        settings_path: mods::settings_path(&s.server_dir).display().to_string(),
        global_enable: ms.global_enable,
        root_exists: root.as_ref().is_some_and(|r| r.is_dir()),
        mods: root.as_ref().map(|r| mods::scan(r, &s.server_dir, &ms.active)).unwrap_or_default(),
        workshop_root: root.map(|r| r.display().to_string()),
        candidates: candidates.iter().map(|c| c.display().to_string()).collect(),
        download_root: mods::download_root(&s.server_dir).display().to_string(),
    }
}

#[tauri::command]
pub async fn mods_state(st: S<'_>) -> Result<ModsState> { Ok(mods_state_for(&*st.settings.read().await)) }

#[tauri::command]
pub async fn mods_set_global(st: S<'_>, enabled: bool) -> Result<()> {
    let s = st.settings.read().await.clone();
    let mut ms = mods::load_settings(&s.server_dir);
    ms.global_enable = enabled;
    mods::save_settings(&s.server_dir, &ms)
}

/// Définit le dossier Workshop lu par le serveur (doit exister).
#[tauri::command]
pub async fn mods_set_root(st: S<'_>, path: String) -> Result<()> {
    let p = std::path::PathBuf::from(path.trim().trim_matches('"'));
    if !p.is_dir() { return Err(Error::Other(format!("dossier introuvable : {}", p.display()))); }
    let s = st.settings.read().await.clone();
    let mut ms = mods::load_settings(&s.server_dir);
    ms.workshop_root = Some(p.display().to_string());
    mods::save_settings(&s.server_dir, &ms)
}

#[tauri::command]
pub async fn mods_set_active(st: S<'_>, package_name: String, active: bool) -> Result<()> {
    let s = st.settings.read().await.clone();
    let mut ms = mods::load_settings(&s.server_dir);
    // On n'active que des mods réellement présents dans le dossier Workshop courant.
    if active {
        let root = ms.workshop_root.clone().map(std::path::PathBuf::from).or_else(|| mods::candidate_roots(&s.server_dir).into_iter().next());
        let known = root.is_some_and(|r| mods::scan(&r, &s.server_dir, &[]).iter().any(|m| m.package_name == package_name));
        if !known { return Err(Error::Other("mod introuvable dans le dossier Workshop".into())); }
    }
    mods::set_active(&mut ms, &package_name, active);
    mods::save_settings(&s.server_dir, &ms)
}

/// Télécharge un mod du Workshop (identifiant ou adresse) via SteamCMD, dans le dossier du serveur.
/// Si aucun dossier Workshop n'est encore défini, celui du serveur est adopté.
#[tauri::command]
pub async fn mods_add(st: S<'_>, input: String) -> Result<String> {
    st.audit.record("app", "mod ajouté", &input);
    let id = mods::parse_workshop_id(&input).ok_or_else(|| Error::Other("identifiant ou adresse Workshop invalide".into()))?;
    let s = st.settings.read().await.clone();
    steamcmd::download_workshop(&s.steamcmd_exe(), &s.server_dir, &id).await?;
    let dl = mods::download_root(&s.server_dir);
    let mut ms = mods::load_settings(&s.server_dir);
    let note = match &ms.workshop_root {
        None => { ms.workshop_root = Some(dl.display().to_string()); mods::save_settings(&s.server_dir, &ms)?; String::new() }
        Some(r) if std::path::Path::new(r) != dl.as_path() =>
            format!(" Attention : le serveur lit actuellement {r} ; ce mod est dans {}. Choisissez un seul dossier Workshop.", dl.display()),
        _ => String::new(),
    };
    // Mod géré : tenu à jour automatiquement, et activé s'il fonctionne sur un serveur dédié.
    let mut s2 = st.settings.write().await;
    if !s2.mod_automation.managed_ids.contains(&id) { s2.mod_automation.managed_ids.push(id.clone()); }
    let _ = s2.save(&st.settings_path);
    drop(s2);
    let ms = mods::load_settings(&s.server_dir);
    let pkg = mods::scan(&dl, &s.server_dir, &ms.active).into_iter().find(|m| m.workshop_id == id);
    let activated = match pkg {
        Some(m) if m.server_compatible => { let mut ms = ms; mods::set_active(&mut ms, &m.package_name, true); mods::save_settings(&s.server_dir, &ms)?; " Activé." }
        Some(_) => " Non activé : ce mod n'est pas compatible serveur dédié.",
        None => "",
    };
    Ok(format!("Mod {id} téléchargé.{activated}{note} Redémarrez le serveur pour le charger."))
}

/// Retire un mod téléchargé par l'application (dossier du serveur) et le désactive.
#[tauri::command]
pub async fn mods_remove(st: S<'_>, workshop_id: String) -> Result<()> {
    st.audit.record("app", "mod retiré", &workshop_id);
    let s = st.settings.read().await.clone();
    let mut ms = mods::load_settings(&s.server_dir);
    let pkg = mods::scan(&mods::download_root(&s.server_dir), &s.server_dir, &[]).into_iter().find(|m| m.workshop_id == workshop_id).map(|m| m.package_name);
    mods::remove_downloaded(&s.server_dir, &workshop_id)?;
    { let mut w = st.settings.write().await; w.mod_automation.managed_ids.retain(|i| i != &workshop_id); let _ = w.save(&st.settings_path); }
    if let Some(p) = pkg { mods::set_active(&mut ms, &p, false); mods::save_settings(&s.server_dir, &ms)?; }
    Ok(())
}

/// Analyse la fin du journal du serveur (≈ 64 Ko) et renvoie les causes probables de problèmes.
#[tauri::command]
pub async fn analyze_log(st: S<'_>) -> Result<Vec<palmanager_core::loganalysis::Finding>> {
    let s = st.settings.read().await;
    let chunk = logs::read_located(&s.server_dir, s.log_file.as_deref(), st.server.console_log_path(), None)?;
    Ok(palmanager_core::loganalysis::analyze(&chunk.lines))
}

/// Version de l'application en cours d'exécution.
#[tauri::command]
pub fn app_version() -> String { palmanager_core::update::current_version_string() }

/// Interroge les Releases GitHub ; `None` si l'application est à jour.
#[tauri::command]
pub async fn check_update() -> Result<Option<palmanager_core::update::UpdateInfo>> {
    palmanager_core::update::check().await
}

/// Télécharge l'installeur, le lance puis ferme l'application (l'installeur remplace les fichiers).
/// Refusé en développement : la mise à jour passe alors par `git pull`.
#[tauri::command]
pub async fn install_update(app: tauri::AppHandle, info: palmanager_core::update::UpdateInfo) -> Result<()> {
    if cfg!(debug_assertions) { return Err(palmanager_core::Error::Other("mise à jour automatique indisponible en mode développement".into())); }
    let path = palmanager_core::update::download(&info).await?;
    std::process::Command::new(&path).spawn().map_err(palmanager_core::Error::from)?;
    app.exit(0);
    Ok(())
}

/// Contrôle qu'une sauvegarde est lisible et complète (sans rien restaurer).
#[tauri::command]
pub async fn verify_backup(st: S<'_>, path: String) -> Result<backup::VerifyReport> {
    let dest = st.settings.read().await.backup.destination.canonicalize()?;
    let p = std::path::PathBuf::from(&path).canonicalize()?;
    if !p.starts_with(dest) { return Err(Error::Other("archive hors du dossier de sauvegarde".into())); }
    tokio::task::spawn_blocking(move || backup::verify(&p)).await.map_err(|e| Error::Other(e.to_string()))?
}

#[tauri::command]
pub fn profiles_list(st: S<'_>) -> Vec<palmanager_core::profiles::ProfileInfo> { st.profiles.list() }

/// Enregistre les réglages actuels du monde sous ce nom.
#[tauri::command]
pub async fn profile_save(st: S<'_>, name: String) -> Result<()> {
    let (options, _) = st.settings.read().await.load_world_options()?;
    st.profiles.save(&name, &options)
}

/// Applique un profil sur PalWorldSettings.ini (copie `.ini.bak`). Renvoie le nombre de valeurs modifiées ; redémarrage du serveur requis.
#[tauri::command]
pub async fn profile_apply(st: S<'_>, name: String) -> Result<usize> {
    st.audit.record("app", "profil appliqué", &name);
    let s = st.settings.read().await.clone();
    crate::automation::apply_profile(&st, &s, &name)
}

#[tauri::command]
pub fn profile_delete(st: S<'_>, name: String) -> Result<()> { st.profiles.delete(&name) }

#[derive(serde::Serialize)]
pub struct ServerUpdateInfo { pub installed: Option<String>, pub latest: String, pub outdated: bool }

/// Compare le build installé au build public Steam.
#[tauri::command]
pub async fn check_server_update(st: S<'_>) -> Result<ServerUpdateInfo> {
    let s = st.settings.read().await.clone();
    let latest = steamcmd::latest_build(&s.steamcmd_exe()).await?;
    let installed = steamcmd::installed_build(&s.server_dir);
    let outdated = installed.as_deref().is_some_and(|i| i != latest);
    Ok(ServerUpdateInfo { installed, latest, outdated })
}

/// Adopte le dossier d'installation du serveur actuellement en marche comme dossier configuré.
#[tauri::command]
pub async fn use_running_server_dir(app: tauri::AppHandle, st: S<'_>) -> Result<String> {
    let dir = palmanager_core::server::running_server_dir().ok_or_else(|| Error::Other("aucun serveur en cours d'exécution".into()))?;
    let mut s = st.settings.read().await.clone();
    s.server_dir = dir.clone();
    s.save(&st.settings_path)?;
    *st.settings.write().await = s;
    crate::remote::apply(&app).await;
    Ok(dir.display().to_string())
}

/// Enregistre l'ensemble actuel des mods activés comme pack.
#[tauri::command]
pub async fn mod_pack_save(st: S<'_>, name: String) -> Result<()> {
    let name = name.trim().chars().take(40).collect::<String>();
    if name.is_empty() { return Err(Error::Other("nom de pack vide".into())); }
    let mut s = st.settings.write().await;
    let active = mods::load_settings(&s.server_dir).active;
    s.mod_automation.packs.retain(|p| p.name != name);
    s.mod_automation.packs.push(palmanager_core::settings::ModPack { name, package_names: active });
    s.save(&st.settings_path)
}

/// Active exactement les mods du pack (ceux qui existent et fonctionnent sur un serveur dédié). Renvoie ceux qui manquent.
#[tauri::command]
pub async fn mod_pack_apply(st: S<'_>, name: String) -> Result<Vec<String>> {
    let s = st.settings.read().await.clone();
    let pack = s.mod_automation.packs.iter().find(|p| p.name == name).ok_or_else(|| Error::Other("pack introuvable".into()))?.clone();
    let mut ms = mods::load_settings(&s.server_dir);
    let root = ms.workshop_root.clone().map(std::path::PathBuf::from).or_else(|| mods::candidate_roots(&s.server_dir).into_iter().next()).unwrap_or_else(|| mods::download_root(&s.server_dir));
    let available = mods::scan(&root, &s.server_dir, &[]);
    let missing = mods::apply_pack(&mut ms, &pack.package_names, &available);
    mods::save_settings(&s.server_dir, &ms)?;
    Ok(missing)
}

#[tauri::command]
pub async fn mod_pack_delete(st: S<'_>, name: String) -> Result<()> {
    let mut s = st.settings.write().await;
    s.mod_automation.packs.retain(|p| p.name != name);
    s.save(&st.settings_path)
}

/// Installation complète depuis zéro : SteamCMD puis le serveur Palworld, avec progression (événement `install-log`).
/// L'application est ensuite configurée sur ces dossiers. Ne touche à rien d'existant hors du dossier choisi.
#[tauri::command]
pub async fn install_everything(app: tauri::AppHandle, st: S<'_>, base_dir: String) -> Result<()> {
    use tauri::Emitter;
    let base = std::path::PathBuf::from(base_dir.trim().trim_matches('"'));
    if base.as_os_str().is_empty() || !base.is_absolute() { return Err(Error::Other("indiquez un dossier absolu (ex. C:\\palworld)".into())); }
    if st.maintenance.swap(true, Ordering::SeqCst) { return Err(Error::Other("une maintenance est déjà en cours".into())); }
    let emit = |m: &str| { let _ = app.emit("install-log", m.to_string()); };
    let res: Result<()> = async {
        std::fs::create_dir_all(&base)?;
        if palmanager_core::health::free_space(&base).is_some_and(|f| f < palmanager_core::install::MIN_FREE_BYTES) {
            return Err(Error::Other("espace disque insuffisant (12 Go libres conseillés)".into()));
        }
        emit("Téléchargement de SteamCMD…");
        let steamcmd_dir = base.join("steamcmd");
        let exe = palmanager_core::install::download_steamcmd(&steamcmd_dir).await?;
        emit("Première exécution de SteamCMD (mise à jour automatique)…");
        palmanager_core::install::run_streaming(&exe, &["+quit".to_string()], |l| emit(&l)).await?;
        emit("Installation du serveur Palworld (plusieurs Go, patience)…");
        let server_dir = base.join("PalServer");
        palmanager_core::install::install_server(&exe, &server_dir, |l| emit(&l)).await?;
        let mut s = st.settings.write().await;
        s.server_dir = server_dir;
        s.steamcmd_path = exe;
        s.save(&st.settings_path)?;
        emit("Terminé : serveur installé et application configurée.");
        Ok(())
    }.await;
    st.maintenance.store(false, Ordering::SeqCst);
    res
}

/// Essaie d'ouvrir le port de jeu via UPnP (test manuel depuis l'interface) ; renvoie l'adresse publique vue par la box.
#[tauri::command]
pub async fn upnp_test(st: S<'_>) -> Result<String> {
    let s = st.settings.read().await.clone();
    let port = palmanager_core::upnp::game_port(s.load_world_options().ok().and_then(|(o, _)| ini::get(&o, "PublicPort").map(String::from)).as_deref());
    let ip = palmanager_core::net::lan_ip().ok_or_else(|| Error::Other("adresse du PC sur le réseau local introuvable".into()))?;
    let public = palmanager_core::upnp::open(port, ip).await?;
    Ok(format!("Port UDP {port} ouvert vers {ip}. Adresse publique vue par la box : {public}"))
}

/// Sauvegardes individuelles d'un joueur (la plus récente d'abord).
#[tauri::command]
pub async fn player_snapshots(st: S<'_>, player_id: String) -> Result<Vec<palmanager_core::playersaves::PlayerSnapshot>> {
    let dest = st.settings.read().await.backup.destination.clone();
    Ok(palmanager_core::playersaves::list(&dest, &player_id))
}

/// Sauvegarde maintenant les fichiers du joueur (après avoir demandé au serveur d'écrire le monde sur disque).
#[tauri::command]
pub async fn player_snapshot_now(st: S<'_>, player_id: String) -> Result<Option<palmanager_core::playersaves::PlayerSnapshot>> {
    let s = st.settings.read().await.clone();
    if let Ok(api) = RestClient::new(&s.rest) { let _ = api.save().await; }
    tokio::time::sleep(Duration::from_secs(2)).await;
    tokio::task::spawn_blocking(move || palmanager_core::playersaves::snapshot_player(&s.save_dir(), &s.backup.destination, &player_id, s.backup.player_keep))
        .await.map_err(|e| Error::Other(e.to_string()))?
}

/// Restaure la fiche d'un joueur (même monde). Le serveur est arrêté (avec sauvegarde du monde), les fichiers sont remplacés,
/// puis le serveur est relancé s'il tournait. Les objets, Pals et bases du joueur ne sont PAS concernés (voir `playersaves`).
#[tauri::command]
pub async fn player_restore(st: S<'_>, path: String) -> Result<Vec<String>> {
    st.audit.record("app", "restauration d'une fiche joueur", &path);
    let s = st.settings.read().await.clone();
    let root = s.backup.destination.join("joueurs").canonicalize()?;
    let p = std::path::PathBuf::from(&path).canonicalize()?;
    if !p.starts_with(&root) { return Err(Error::Other("archive hors du dossier des sauvegardes joueur".into())); }
    if st.maintenance.swap(true, Ordering::SeqCst) { return Err(Error::Other("une maintenance est déjà en cours".into())); }
    let was_running = st.server.is_running().await;
    let res: Result<Vec<String>> = async {
        if was_running {
            st.expected_stop.store(true, Ordering::SeqCst);
            st.server.stop(&s, Duration::from_secs(60)).await?; // sauvegarde du monde si backup.on_stop
        }
        let dir = s.save_dir();
        tokio::task::spawn_blocking(move || palmanager_core::playersaves::restore_player(&p, &dir)).await.map_err(|e| Error::Other(e.to_string()))?
    }.await;
    if was_running {
        st.expected_stop.store(false, Ordering::SeqCst);
        let _ = st.server.start(&s).await;
    }
    st.maintenance.store(false, Ordering::SeqCst);
    res
}

/// Copie une sauvegarde joueur (zip) vers un dossier choisi (par défaut le Bureau) ; renvoie le fichier créé.
#[tauri::command]
pub async fn player_export(st: S<'_>, path: String, target_dir: Option<String>) -> Result<String> {
    let s = st.settings.read().await.clone();
    let root = s.backup.destination.join("joueurs").canonicalize()?;
    let p = std::path::PathBuf::from(&path).canonicalize()?;
    if !p.starts_with(&root) { return Err(Error::Other("archive hors du dossier des sauvegardes joueur".into())); }
    let dir = target_dir.filter(|d| !d.trim().is_empty()).map(|d| std::path::PathBuf::from(d.trim().trim_matches('"')))
        .or_else(|| std::env::var_os("USERPROFILE").map(|h| std::path::PathBuf::from(h).join("Desktop")))
        .ok_or_else(|| Error::Other("indiquez un dossier de destination".into()))?;
    std::fs::create_dir_all(&dir)?;
    let id = p.parent().and_then(|d| d.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let out = dir.join(format!("joueur-{id}-{}", p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
    std::fs::copy(&p, &out)?;
    Ok(out.display().to_string())
}

// ───────────── Audit, verrou, test de restauration, saisons ─────────────

#[tauri::command]
pub fn audit_recent(st: S<'_>, limit: Option<usize>) -> Vec<palmanager_core::audit::AuditEntry> { st.audit.recent(limit.unwrap_or(200).min(1000)) }

#[derive(serde::Serialize)]
pub struct LockStatus { pub enabled: bool, pub auto_lock_minutes: u32 }

#[tauri::command]
pub async fn lock_status(st: S<'_>) -> Result<LockStatus> {
    let l = st.settings.read().await.lock.clone();
    Ok(LockStatus { enabled: l.enabled, auto_lock_minutes: l.auto_lock_minutes })
}

/// Vérifie le code. Chaque échec ralentit la réponse (1 s) pour décourager les essais en rafale.
#[tauri::command]
pub async fn lock_verify(st: S<'_>, pin: String) -> Result<bool> {
    let l = st.settings.read().await.lock.clone();
    let ok = palmanager_core::lock::verify(&l, &pin);
    if !ok { tokio::time::sleep(Duration::from_secs(1)).await; st.audit.record("app", "verrou : code refusé", ""); }
    Ok(ok)
}

/// Définit, change ou retire le code (`new_pin` vide = retirer). Si un code existe, l'ancien est exigé.
#[tauri::command]
pub async fn lock_set(st: S<'_>, current: Option<String>, new_pin: Option<String>, auto_lock_minutes: u32) -> Result<()> {
    let mut s = st.settings.write().await;
    if s.lock.enabled && !palmanager_core::lock::verify(&s.lock, &current.unwrap_or_default()) { return Err(Error::Other("code actuel incorrect".into())); }
    match new_pin.filter(|p| !p.is_empty()) {
        Some(p) => palmanager_core::lock::set_pin(&mut s.lock, &p)?,
        None => palmanager_core::lock::clear(&mut s.lock),
    }
    s.lock.auto_lock_minutes = auto_lock_minutes.min(24 * 60);
    s.save(&st.settings_path)?;
    drop(s);
    st.audit.record("app", "verrou : réglage modifié", "");
    Ok(())
}

#[derive(serde::Serialize)]
pub struct RestoreTestView { pub t: i64, pub ok: bool, pub detail: String }

#[tauri::command]
pub fn restore_test_status(st: S<'_>) -> RestoreTestView {
    let r = palmanager_core::restoretest::load_state(&st.restore_test_path);
    RestoreTestView { t: r.t, ok: r.ok, detail: r.detail }
}

/// Lance le test de restauration maintenant (la dernière sauvegarde est extraite dans un dossier temporaire, puis supprimée).
#[tauri::command]
pub async fn restore_test_now(st: S<'_>) -> Result<RestoreTestView> {
    let s = st.settings.read().await.clone();
    let st2 = crate::supervisor::run_restore_test(&s, &st.restore_test_path).await;
    Ok(RestoreTestView { t: st2.t, ok: st2.ok, detail: st2.detail })
}

/// Événement de saison actuellement appliqué (s'il y en a un).
#[tauri::command]
pub fn season_status(st: S<'_>) -> palmanager_core::season::SeasonState { palmanager_core::season::load_state(&st.season_path) }
