//! Boucle de fond : échantillonnage (5 s), alertes, planification des redémarrages, backups planifiés,
//! redémarrage auto après crash. Émet l'événement `snapshot` vers le frontend à chaque tick.

use crate::{automation, state::AppState};
use palmanager_core::{alerts, health, steamcmd, announce, backup, players, perf, server::find_server_pids, history::{PlayerEvent, Sample}, summary, rest::RestClient, schedule::Action, settings::{AppSettings, RuleAction}, watchdog::Watchdog};
use std::{collections::HashSet, sync::atomic::Ordering, time::{Duration, Instant}};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

/// Cycle de maintenance sûr (voir `automation::guarded_cycle`) puis notification du résultat.
async fn run_cycle(app: &AppHandle, st: &AppState, s: &AppSettings, label: &str, server_update: bool, suspect: &[String], profile: Option<&str>) {
    let msg = automation::guarded_cycle(st, s, label, server_update, suspect, profile).await;
    toast(app, s, &msg);
    let _ = alerts::dispatch_text(&s.alerts, &msg).await;
}

/// Arrêt planifié : sauvegarde puis arrêt propre. `expected_stop` reste vrai pour éviter une fausse alerte de
/// crash et un redémarrage automatique ; il est remis à faux au prochain démarrage.
async fn maintenance_stop(st: &AppState, s: &AppSettings) {
    if st.maintenance.swap(true, Ordering::SeqCst) { return; }
    if let Ok(api) = RestClient::new(&s.rest) { let _ = api.save().await; }
    st.expected_stop.store(true, Ordering::SeqCst);
    let _ = st.server.stop(s, Duration::from_secs(60)).await; // sauvegarde incluse (backup.on_stop)
    st.maintenance.store(false, Ordering::SeqCst);
}

async fn scheduled_start(st: &AppState, s: &AppSettings) {
    st.expected_stop.store(false, Ordering::SeqCst);
    let _ = st.server.start(s).await;
}

/// Notification Windows (toast). Les erreurs sont ignorées : une notification manquée ne doit jamais gêner la surveillance.
fn toast(app: &AppHandle, s: &AppSettings, body: &str) {
    if s.alerts.desktop { let _ = app.notification().builder().title("Palworld Server Manager").body(body).show(); }
}

/// Lance une sauvegarde complète hors du thread async ; renvoie le message d'erreur le cas échéant.
async fn run_backup(s: &AppSettings, label: &'static str) -> Option<String> {
    let s2 = s.clone();
    match tokio::task::spawn_blocking(move || backup::backup_all(&s2, Some(label))).await {
        Ok(Ok(_)) => None,
        Ok(Err(e)) => Some(e.to_string()),
        Err(e) => Some(e.to_string()),
    }
}

/// Alerte (Discord/ntfy) en cas d'échec de sauvegarde, au plus une fois par heure pour ne pas spammer.
async fn warn_backup(s: &AppSettings, last: &mut Option<Instant>, what: &str, err: &str) {
    eprintln!("sauvegarde ({what}) en échec : {err}");
    if last.is_some_and(|t| t.elapsed() < Duration::from_secs(3600)) { return; }
    *last = Some(Instant::now());
    let _ = alerts::dispatch_text(&s.alerts, &format!("⚠️ Sauvegarde ({what}) impossible : {err}")).await;
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_backup = Instant::now();
        let mut daily = palmanager_core::schedule::DailyTrigger::new();
        let started_at = Instant::now();
        let mut launch_start_done = false;
        let mut prev_running = false;
        let mut welcome_ready = false;
        let mut last_book_save = 0i64;
        let mut kicked_at: std::collections::HashMap<String, Instant> = std::collections::HashMap::new();
        let mut announcer = announce::Announcer::new();
        let mut perf_applied: Option<(String, Vec<u32>)> = None; // (réglages, PID déjà traités)
        let mut last_limit_restart: Option<Instant> = None;
        let mut last_backup_alert: Option<Instant> = None;
        let mut last_sample = 0i64;
        let mut wd = Watchdog::new();
        let mut crash_blocked = false;
        let mut prev_up = false;
        let mut empty_wait: Option<Instant> = None;
        let mut last_mod_check: Option<Instant> = None;
        let mut upnp_port: Option<u16> = None;
        let mut last_upnp: Option<Instant> = None;
        let mut upnp_warned = false;
        let mut up_since: Option<Instant> = None;
        let mut last_health_check = Instant::now();
        let mut health_sent: std::collections::HashMap<&'static str, Instant> = std::collections::HashMap::new();
        let mut last_update_check: Option<Instant> = None;
        let mut update_attempted: Option<String> = None;
        let mut known: HashSet<String> = HashSet::new();
        let mut tick = tokio::time::interval(Duration::from_secs(5));
        loop {
            tick.tick().await;
            let st = app.state::<AppState>();
            let s = st.settings.read().await.clone();
            let api = RestClient::new(&s.rest).ok();
            let maintenance = st.maintenance.load(Ordering::SeqCst);

            let snap = st.monitor.lock().await.sample(st.server.pid(), api.as_ref()).await;
            let expected = st.expected_stop.load(Ordering::SeqCst) || maintenance;
            // Démarrage automatique du serveur ~15 s après le lancement de l'appli (laisse Windows finir de démarrer).
            if !launch_start_done && started_at.elapsed() >= Duration::from_secs(15) {
                launch_start_done = true;
                if s.start_server_on_launch && !snap.running && !maintenance { scheduled_start(&st, &s).await; }
            }

            // Arrêt non demandé depuis l'application (fenêtre du serveur fermée, crash…) : sauvegarde du monde figé,
            // avant toute relance automatique. Nos propres arrêts sont déjà sauvegardés par `ServerController::stop`.
            if prev_running && !snap.running && !expected && s.backup.on_stop {
                if let Some(e) = run_backup(&s, "arret-externe").await { warn_backup(&s, &mut last_backup_alert, "arrêt du serveur", &e).await; }
            }
            prev_running = snap.running;
            if let Some(e) = st.server.take_backup_warning() { warn_backup(&s, &mut last_backup_alert, "arrêt du serveur", &e).await; }

            let events = st.alerts.lock().await.evaluate(&s.alerts, &snap, expected);
            for e in &events {
                // Crash : on ajoute la cause probable tirée de la fin du journal, si on en reconnaît une.
                if matches!(e, alerts::Event::Crash) {
                    let cause = palmanager_core::logs::read_located(&s.server_dir, s.log_file.as_deref(), st.server.console_log_path(), None).ok().and_then(|c| palmanager_core::loganalysis::probable_cause(&c.lines));
                    let msg = match cause { Some(c) => format!("{} Cause probable : {c}.", e.message()), None => e.message() };
                    toast(&app, &s, &msg);
                    let _ = alerts::dispatch_text(&s.alerts, &msg).await;
                } else {
                    toast(&app, &s, &e.message());
                    let _ = alerts::dispatch(&s.alerts, e).await;
                }
                if matches!(e, alerts::Event::Crash) && s.auto_restart && !crash_blocked {
                    if wd.record_crash(&s.watchdog, chrono::Utc::now().timestamp()) {
                        crash_blocked = true;
                        let m = format!("🛑 {} crashs en {} min : relance automatique arrêtée pour ne pas boucler. Consultez l'onglet Journal puis démarrez le serveur à la main.", s.watchdog.crash_loop_max, s.watchdog.crash_loop_window_minutes);
                        toast(&app, &s, &m);
                        let _ = alerts::dispatch_text(&s.alerts, &m).await;
                    } else { let _ = st.server.start(&s).await; }
                }
            }

            // Horaires programmés (démarrer / arrêter / redémarrer) et redémarrage préventif sur seuil de RAM.
            if !maintenance {
                let now = chrono::Local::now().naive_local();
                let actions = st.scheduler.lock().await.tick(&s.schedule, now);
                for a in actions {
                    match a {
                        Action::Announce { minutes, action } => {
                            if let (true, Some(api)) = (snap.running, &api) {
                                let what = if action == RuleAction::Stop { "Arrêt" } else { "Redémarrage" };
                                let _ = api.announce(&format!("{what} du serveur dans {minutes} minute(s).")).await;
                            }
                        }
                        Action::Run { action: RuleAction::Start, profile } => if !snap.running {
                            if profile.is_some() { run_cycle(&app, &st, &s, "planifie", false, &[], profile.as_deref()).await } else { scheduled_start(&st, &s).await }
                        },
                        Action::Run { action: RuleAction::Stop, .. } => if snap.running { maintenance_stop(&st, &s).await },
                        Action::Run { action: RuleAction::Restart, profile } => run_cycle(&app, &st, &s, "planifie", false, &[], profile.as_deref()).await,
                    }
                }
                if let Some(th) = s.schedule.memory_restart_percent.filter(|_| s.schedule.enabled && snap.running) {
                    if snap.memory_percent >= th && empty_wait.is_none() {
                        let wait = s.schedule.memory_restart_wait_empty_minutes;
                        if wait > 0 && !snap.players.is_empty() {
                            if let Some(api) = &api { let _ = api.announce("Mémoire élevée : redémarrage dès que le serveur sera vide.").await; }
                            empty_wait = Some(Instant::now() + Duration::from_secs(wait * 60));
                        } else {
                            if let Some(api) = &api { let _ = api.announce("Mémoire élevée : redémarrage dans 1 minute.").await; }
                            tokio::time::sleep(Duration::from_secs(60)).await;
                            run_cycle(&app, &st, &s, "memoire", false, &[], None).await;
                        }
                    }
                }
                if let Some(deadline) = empty_wait {
                    if !snap.running { empty_wait = None; }
                    else if snap.players.is_empty() || Instant::now() >= deadline {
                        empty_wait = None;
                        if !snap.players.is_empty() {
                            if let Some(api) = &api { let _ = api.announce("Redémarrage dans 1 minute (mémoire élevée).").await; }
                            tokio::time::sleep(Duration::from_secs(60)).await;
                        }
                        run_cycle(&app, &st, &s, "memoire", false, &[], None).await;
                    }
                }
            }

            // Priorité / cœurs : appliqués à chaque nouveau processus serveur (démarrage, relance, serveur adopté)
            // et dès que les réglages changent. Aucun appel PowerShell tant que les réglages restent ceux par défaut.
            if snap.running {
                let sig = serde_json::to_string(&s.performance).unwrap_or_default();
                let mut pids = find_server_pids();
                pids.sort_unstable();
                let changed = match &perf_applied { Some((old_sig, old)) => *old_sig != sig || *old != pids, None => true };
                if changed {
                    let is_default = s.performance == Default::default();
                    if !(is_default && perf_applied.is_none()) {
                        let (p2, cores) = (s.performance.clone(), palmanager_core::monitor::system_info().cpu_cores);
                        let _ = tokio::task::spawn_blocking(move || perf::apply(&p2, cores)).await;
                    }
                    perf_applied = Some((sig, pids));
                }
            }

            // Limite de RAM du serveur (réglée dans l'onglet Performance) : alerte, ou redémarrage avec annonce d'1 minute,
            // au plus un redémarrage par 30 minutes pour éviter une boucle si la limite est trop basse.
            if let (Some(limit), true, false) = (s.performance.memory_limit_gb, snap.running, maintenance) {
                let used_gb = snap.memory_bytes as f64 / 1e9;
                if used_gb >= limit as f64 && last_limit_restart.is_none_or(|t| t.elapsed() >= Duration::from_secs(1800)) {
                    last_limit_restart = Some(Instant::now());
                    let txt = format!("🟠 RAM du serveur : {used_gb:.1} Go (limite {limit:.1} Go){}", if s.performance.memory_limit_restart { " — redémarrage dans 1 minute." } else { "." });
                    let _ = alerts::dispatch_text(&s.alerts, &txt).await;
                    if s.performance.memory_limit_restart {
                        if let Some(api) = &api { let _ = api.announce("Mémoire élevée : redémarrage dans 1 minute.").await; }
                        tokio::time::sleep(Duration::from_secs(60)).await;
                        run_cycle(&app, &st, &s, "memoire", false, &[], None).await;
                    }
                }
            }

            if s.backup.enabled && snap.running && last_backup.elapsed() >= Duration::from_secs(s.backup.interval_minutes.max(1) * 60) {
                if let Some(api) = &api { let _ = api.save().await; }
                if let Some(e) = run_backup(&s, "auto").await { warn_backup(&s, &mut last_backup_alert, "automatique", &e).await; }
                last_backup = Instant::now();
            }

            // Historique : un échantillon toutes les 30 s + événements de connexion.
            let ts = chrono::Utc::now().timestamp();
            if snap.running && ts - last_sample >= 30 {
                let _ = st.history.add_sample(&Sample::from_snapshot(ts, &snap));
                last_sample = ts;
            }
            let now: HashSet<String> = snap.players.iter().map(|p| p.name.clone()).collect();
            if !snap.running || snap.metrics.is_some() { // ignore les instantanés où l'API REST n'a pas répondu
                for n in now.difference(&known) {
                    let _ = st.history.add_event(&PlayerEvent { t: ts, name: n.clone(), joined: true });
                    // Bienvenue aux nouveaux arrivants. Pas au premier instantané après le lancement de l'application :
                    // les joueurs déjà connectés ne sont pas « nouveaux ».
                    if welcome_ready && s.announcements.enabled {
                        if let (Some(tpl), Some(api)) = (s.announcements.welcome.as_deref().filter(|t| !t.trim().is_empty()), &api) {
                            let max = snap.metrics.as_ref().map(|m| m.maxplayernum);
                            let _ = api.announce(&announce::render(tpl, Some(n), snap.players.len(), max)).await;
                        }
                    }
                }
                for n in known.difference(&now) { let _ = st.history.add_event(&PlayerEvent { t: ts, name: n.clone(), joined: false }); }
                known = now;
                if snap.running { welcome_ready = true; }
            }

            // Carnet des joueurs (temps de jeu, sessions) et liste blanche : uniquement quand l'API a répondu,
            // pour ne jamais agir sur une liste de joueurs incertaine.
            if snap.running && snap.metrics.is_some() {
                let current: Vec<(String, String)> = snap.players.iter().map(|p| (p.user_id.clone(), p.name.clone())).collect();
                let mut book = st.players.lock().await;
                book.observe(ts, &current);
                if ts - last_book_save >= 60 { let _ = book.save_if_dirty(); last_book_save = ts; }
                drop(book);
                if let Some(api) = &api {
                    for p in &snap.players {
                        let recently = kicked_at.get(&p.user_id).is_some_and(|t| t.elapsed() < Duration::from_secs(20));
                        if !recently && players::should_kick(&s.access, &p.user_id) {
                            kicked_at.insert(p.user_id.clone(), Instant::now());
                            let msg = if s.access.kick_message.trim().is_empty() { "Ce serveur est privé (liste blanche)." } else { s.access.kick_message.as_str() };
                            let _ = api.kick(&p.user_id, msg).await;
                        }
                    }
                }
            }

            // Santé des sauvegardes (toutes les 5 min) : trop ancienne, ou disque presque plein. Une alerte par type et par 6 h.
            if snap.running { if up_since.is_none() { up_since = Some(Instant::now()); } } else { up_since = None; }
            if last_health_check.elapsed() >= Duration::from_secs(300) {
                last_health_check = Instant::now();
                let (dest, cfg) = (s.backup.destination.clone(), s.alerts.clone());
                let up = up_since.map_or(0, |t| t.elapsed().as_secs() as i64);
                let (running, backups_on) = (snap.running, s.backup.enabled);
                let issues = tokio::task::spawn_blocking(move || {
                    let backups = backup::list(&dest).unwrap_or_default();
                    let mut v = health::evaluate(&cfg, &backups, chrono::Local::now(), running && backups_on, up, health::free_space(&dest));
                    v.retain(|i| backups_on || matches!(i, health::Issue::LowDisk { .. }));
                    v
                }).await.unwrap_or_default();
                for i in issues {
                    if health_sent.get(i.key()).is_some_and(|t| t.elapsed() < Duration::from_secs(6 * 3600)) { continue; }
                    health_sent.insert(i.key(), Instant::now());
                    toast(&app, &s, &i.message());
                    let _ = alerts::dispatch_text(&s.alerts, &i.message()).await;
                }
            }

            // Mise à jour automatique du serveur Palworld : build Steam public ≠ build installé.
            if s.server_update.enabled && !maintenance
                && last_update_check.map_or(started_at.elapsed() >= Duration::from_secs(120), |t| t.elapsed() >= Duration::from_secs(s.server_update.check_every_minutes.max(15) * 60))
            {
                last_update_check = Some(Instant::now());
                if let Ok(latest) = steamcmd::latest_build(&s.steamcmd_exe()).await {
                    let installed = steamcmd::installed_build(&s.server_dir);
                    if installed.as_deref().is_some_and(|i| i != latest) && update_attempted.as_deref() != Some(latest.as_str()) {
                        update_attempted = Some(latest.clone());
                        let wait = s.server_update.warn_minutes.min(30);
                        if snap.running && wait > 0 && !snap.players.is_empty() {
                            if let Some(api) = &api { let _ = api.announce(&format!("Mise à jour du serveur dans {wait} minute(s).")).await; }
                            tokio::time::sleep(Duration::from_secs(wait * 60)).await;
                        }
                        let _ = alerts::dispatch_text(&s.alerts, &format!("🔄 Mise à jour du serveur Palworld (build {} → {latest}).", installed.unwrap_or_default())).await;
                        run_cycle(&app, &st, &s, "maj", true, &[], None).await;
                    }
                }
            }

            // Surveillance : serveur gelé (API muette après avoir répondu) → redémarrage ; relance auto bloquée après une boucle de crashs.
            {
                let now_ts = chrono::Utc::now().timestamp();
                let up = up_since.map_or(0, |t| t.elapsed().as_secs() as i64);
                if wd.observe(&s.watchdog, now_ts, snap.running, snap.metrics.is_some(), up) && !maintenance {
                    let m = format!("🧊 Le serveur ne répond plus depuis {} min (processus vivant) : redémarrage automatique.", s.watchdog.hung_minutes);
                    toast(&app, &s, &m);
                    let _ = alerts::dispatch_text(&s.alerts, &m).await;
                    run_cycle(&app, &st, &s, "gel", false, &[], None).await;
                }
                if snap.running && !prev_up && crash_blocked { crash_blocked = false; wd.reset_crashes(); }
                prev_up = snap.running;
            }

            // Mods gérés : téléchargés et mis à jour automatiquement. Un changement sur un serveur en marche déclenche un
            // redémarrage sûr (annonce, sauvegarde, vérification, désactivation des mods fautifs si le serveur ne repart pas).
            if s.mod_automation.auto_update && !s.mod_automation.managed_ids.is_empty() && !maintenance
                && last_mod_check.map_or(started_at.elapsed() >= Duration::from_secs(180), |t| t.elapsed() >= Duration::from_secs(s.mod_automation.check_every_minutes.max(30) * 60))
            {
                last_mod_check = Some(Instant::now());
                let (changed, errors) = automation::mods_sync(&s).await;
                if !errors.is_empty() {
                    let _ = alerts::dispatch_text(&s.alerts, &format!("⚠️ Mods : {}", errors.join(" ; "))).await;
                }
                if !changed.is_empty() && st.server.is_running().await {
                    let names = automation::package_names(&s, &changed);
                    let wait = s.server_update.warn_minutes.min(30);
                    let _ = alerts::dispatch_text(&s.alerts, &format!("🧩 Mods mis à jour ({}) : redémarrage du serveur.", changed.join(", "))).await;
                    if wait > 0 && !snap.players.is_empty() {
                        if let Some(api) = &api { let _ = api.announce(&format!("Mise à jour de mods : redémarrage dans {wait} minute(s).")).await; }
                        tokio::time::sleep(Duration::from_secs(wait * 60)).await;
                    }
                    run_cycle(&app, &st, &s, "mods", false, &names, None).await;
                }
            }

            // UPnP : ouverture du port de jeu sur la box, uniquement si vous l'avez activée ; renouvelée toutes les 20 min.
            {
                let want = (s.upnp.enabled && snap.running).then(|| palmanager_core::upnp::game_port(s.load_world_options().ok().and_then(|(o, _)| palmanager_core::ini::get(&o, "PublicPort").map(String::from)).as_deref()));
                match (want, upnp_port) {
                    (Some(port), cur) if cur != Some(port) || last_upnp.is_none_or(|t| t.elapsed() >= Duration::from_secs(1200)) => {
                        last_upnp = Some(Instant::now());
                        if let Some(ip) = palmanager_core::net::lan_ip() {
                            match palmanager_core::upnp::open(port, ip).await {
                                Ok(_) => { upnp_port = Some(port); upnp_warned = false; }
                                Err(e) => if !upnp_warned { upnp_warned = true; let _ = alerts::dispatch_text(&s.alerts, &format!("⚠️ UPnP : {e}")).await; },
                            }
                        }
                    }
                    (None, Some(port)) => { let _ = palmanager_core::upnp::close(port).await; upnp_port = None; last_upnp = None; }
                    _ => {}
                }
            }

            // Rappels réguliers (hors maintenance, serveur en ligne et API joignable).
            if snap.running && !maintenance {
                if let (Some(api), Some(m)) = (&api, &snap.metrics) {
                    for text in announcer.due(&s.announcements, ts, snap.players.len(), Some(m.maxplayernum)) {
                        let _ = api.announce(&text).await;
                    }
                }
            }

            // Résumé quotidien (Discord / ntfy) à l'heure choisie.
            if daily.due(s.alerts.daily_summary_time.as_deref(), chrono::Local::now().naive_local()) {
                let since = ts - 86_400;
                let text = summary::build(&st.history.samples_since(since, 10_000), &st.history.events_since(since), ts);
                let _ = alerts::dispatch_text(&s.alerts, &text).await;
            }

            *st.last_snapshot.write().await = snap.clone();
            let _ = app.emit("snapshot", snap);
        }
    });
}
