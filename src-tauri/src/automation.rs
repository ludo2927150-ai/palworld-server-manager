//! Cycles de maintenance « sûrs » : sauvegarde, changement (profil, mise à jour du serveur, mods), redémarrage,
//! vérification que le serveur répond, et retour arrière automatique si ce n'est pas le cas.

use crate::state::AppState;
use palmanager_core::{backup, ini, mods, rest::RestClient, settings::AppSettings, steamcmd, Error, Result};
use std::{sync::atomic::Ordering, time::{Duration, Instant}};

/// Applique un profil de configuration sur PalWorldSettings.ini (copie `.ini.bak`). Renvoie le nombre de valeurs modifiées.
pub fn apply_profile(st: &AppState, s: &AppSettings, name: &str) -> Result<usize> {
    let (mut cur, _) = s.load_world_options()?;
    let n = palmanager_core::profiles::apply(&mut cur, &st.profiles.load(name)?);
    let path = s.world_settings_path();
    if let Some(dir) = path.parent() { std::fs::create_dir_all(dir)?; }
    if path.exists() { std::fs::copy(&path, path.with_extension("ini.bak"))?; }
    std::fs::write(path, ini::serialize(&cur))?;
    Ok(n)
}

/// Attend que le serveur soit réellement utilisable : processus vivant et API REST qui répond. Sans API configurée,
/// on se contente d'un processus encore vivant après 90 s. `false` = le serveur s'est arrêté ou ne répond pas dans le délai.
pub async fn wait_healthy(st: &AppState, s: &AppSettings, max: Duration) -> bool {
    let api = RestClient::new(&s.rest).ok().filter(|_| !s.rest.admin_password.is_empty());
    let start = Instant::now();
    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;
        if !st.server.is_running().await { return false; }
        match &api {
            Some(api) => if api.metrics().await.is_ok() { return true; },
            None => if start.elapsed() >= Duration::from_secs(90) { return true; },
        }
        if start.elapsed() >= max { return false; }
    }
}

async fn make_backup(s: &AppSettings, label: String) -> Option<std::path::PathBuf> {
    let s2 = s.clone();
    tokio::task::spawn_blocking(move || backup::backup_all(&s2, Some(&label))).await.ok()?.ok().map(|b| b.path)
}

fn newest_backup(s: &AppSettings) -> Option<std::path::PathBuf> {
    backup::list(&s.backup.destination).ok()?.into_iter().next().map(|b| b.path)
}

/// Cycle complet. `server_update` : mise à jour SteamCMD pendant l'arrêt ; `suspect_mods` : noms de paquets des mods qui viennent
/// de changer (désactivés en premier si le serveur ne redémarre pas) ; `profile` : profil de configuration à appliquer.
/// Renvoie un message de résultat (préfixé ✅ / ⚠️ / ↩️ / ❌) à envoyer aux alertes.
pub async fn guarded_cycle(st: &AppState, s: &AppSettings, label: &str, server_update: bool, suspect_mods: &[String], profile: Option<&str>) -> String {
    if st.maintenance.swap(true, Ordering::SeqCst) { return "⏳ Une maintenance est déjà en cours.".into(); }
    let msg = cycle(st, s, label, server_update, suspect_mods, profile).await;
    st.expected_stop.store(false, Ordering::SeqCst);
    st.maintenance.store(false, Ordering::SeqCst);
    msg
}

async fn cycle(st: &AppState, s: &AppSettings, label: &str, server_update: bool, suspect: &[String], profile: Option<&str>) -> String {
    let was_running = st.server.is_running().await;
    let mut notes: Vec<String> = Vec::new();
    // Sauvegarde de sûreté : l'arrêt propre en fait déjà une (backup.on_stop) ; sinon on la fait ici.
    let mut safety = if !(was_running && s.backup.on_stop) { make_backup(s, format!("avant-{label}")).await } else { None };
    let mods_snapshot = std::fs::read_to_string(mods::settings_path(&s.server_dir)).ok();
    if was_running {
        if let Ok(api) = RestClient::new(&s.rest) { let _ = api.save().await; }
        st.expected_stop.store(true, Ordering::SeqCst);
        let _ = st.server.stop(s, Duration::from_secs(60)).await;
        if safety.is_none() { safety = newest_backup(s); }
    }
    if let Some(p) = profile {
        match apply_profile(st, s, p) { Ok(n) => notes.push(format!("profil « {p} » appliqué ({n} valeurs)")), Err(e) => notes.push(format!("profil « {p} » non appliqué : {e}")) }
    }
    if server_update {
        match steamcmd::update(&s.steamcmd_exe(), &s.server_dir).await {
            Ok(_) => notes.push("serveur mis à jour".into()),
            Err(e) => { st.expected_stop.store(false, Ordering::SeqCst); if was_running { let _ = st.server.start(s).await; } return format!("❌ Mise à jour du serveur échouée ({e}). Le serveur a été relancé tel quel."); }
        }
    }
    st.expected_stop.store(false, Ordering::SeqCst);
    let suffix = if notes.is_empty() { String::new() } else { format!(" ({})", notes.join(", ")) };
    if st.server.start(s).await.is_err() { return format!("❌ Le serveur n'a pas pu démarrer{suffix}."); }
    if wait_healthy(st, s, Duration::from_secs(360)).await { return format!("✅ Serveur redémarré{suffix}."); }

    // ── Le serveur ne répond pas : retour arrière progressif ──
    st.expected_stop.store(true, Ordering::SeqCst);
    let _ = st.server.stop(s, Duration::from_secs(20)).await;
    let mut ms = mods::load_settings(&s.server_dir);
    if ms.global_enable && !ms.active.is_empty() {
        let off: Vec<String> = if suspect.is_empty() { ms.active.clone() } else { suspect.to_vec() };
        if suspect.is_empty() { ms.global_enable = false; } else { for p in &off { mods::set_active(&mut ms, p, false); } }
        if mods::save_settings(&s.server_dir, &ms).is_ok() {
            st.expected_stop.store(false, Ordering::SeqCst);
            let _ = st.server.start(s).await;
            if wait_healthy(st, s, Duration::from_secs(240)).await {
                return format!("⚠️ Le serveur ne démarrait pas{suffix} : mods désactivés ({}). Réactivez-les un par un dans l'onglet Mods pour trouver le fautif.", off.join(", "));
            }
            st.expected_stop.store(true, Ordering::SeqCst);
            let _ = st.server.stop(s, Duration::from_secs(20)).await;
        }
    }
    if let Some(snap) = &mods_snapshot { let _ = std::fs::write(mods::settings_path(&s.server_dir), snap); }
    if let Some(arch) = safety {
        let dir = s.save_dir();
        let restored = tokio::task::spawn_blocking(move || backup::restore(&arch, &dir)).await.map_err(|e| Error::Other(e.to_string())).and_then(|r| r);
        st.expected_stop.store(false, Ordering::SeqCst);
        let _ = st.server.start(s).await;
        let up = wait_healthy(st, s, Duration::from_secs(240)).await;
        return match (restored, up) {
            (Ok(()), true) => format!("↩️ Le serveur ne démarrait pas{suffix} : monde restauré depuis la sauvegarde de sûreté, serveur de nouveau en ligne."),
            (Ok(()), false) => format!("❌ Monde restauré depuis la sauvegarde de sûreté mais le serveur ne répond toujours pas{suffix} : intervention nécessaire (voir l'onglet Journal)."),
            (Err(e), _) => format!("❌ Le serveur ne démarre pas{suffix} et la restauration a échoué ({e}) : intervention nécessaire."),
        };
    }
    st.expected_stop.store(false, Ordering::SeqCst);
    format!("❌ Le serveur ne répond pas après le redémarrage{suffix} et aucune sauvegarde de sûreté n'était disponible : intervention nécessaire.")
}

/// Télécharge / met à jour les mods gérés. Renvoie (identifiants dont le contenu a changé, erreurs).
pub async fn mods_sync(s: &AppSettings) -> (Vec<String>, Vec<String>) {
    let (mut changed, mut errors) = (Vec::new(), Vec::new());
    for id in &s.mod_automation.managed_ids {
        let dir = mods::download_root(&s.server_dir).join(id);
        let before = mods::dir_signature(&dir);
        match steamcmd::download_workshop(&s.steamcmd_exe(), &s.server_dir, id).await {
            Ok(()) => if mods::dir_signature(&dir) != before { changed.push(id.clone()); },
            Err(e) => errors.push(format!("{id} : {e}")),
        }
    }
    (changed, errors)
}

/// Noms de paquets correspondant à des identifiants Workshop téléchargés dans le dossier du serveur.
pub fn package_names(s: &AppSettings, ids: &[String]) -> Vec<String> {
    mods::scan(&mods::download_root(&s.server_dir), &s.server_dir, &[]).into_iter().filter(|m| ids.contains(&m.workshop_id)).map(|m| m.package_name).collect()
}
