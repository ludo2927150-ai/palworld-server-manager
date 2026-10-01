//! Boucle de fond : échantillonnage (5 s), alertes, planification des redémarrages, backups planifiés,
//! redémarrage auto après crash. Émet l'événement `snapshot` vers le frontend à chaque tick.

use crate::state::AppState;
use palmanager_core::{alerts, backup, perf, server::find_server_pids, history::{PlayerEvent, Sample}, summary, rest::RestClient, schedule::Action, settings::{AppSettings, RuleAction}};
use std::{collections::HashSet, sync::atomic::Ordering, time::{Duration, Instant}};
use tauri::{AppHandle, Emitter, Manager};

/// Sauvegarde puis redémarre proprement (save REST → ZIP → arrêt → démarrage).
async fn maintenance_restart(st: &AppState, s: &AppSettings) {
    if st.maintenance.swap(true, Ordering::SeqCst) { return; }
    if let Ok(api) = RestClient::new(&s.rest) { let _ = api.save().await; }
    st.expected_stop.store(true, Ordering::SeqCst);
    let _ = st.server.stop(s, Duration::from_secs(60)).await; // sauvegarde incluse (backup.on_stop)
    st.expected_stop.store(false, Ordering::SeqCst);
    let _ = st.server.start(s).await;
    st.maintenance.store(false, Ordering::SeqCst);
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
        let mut prev_running = false;
        let mut perf_applied: Option<(String, Vec<u32>)> = None; // (réglages, PID déjà traités)
        let mut last_limit_restart: Option<Instant> = None;
        let mut last_backup_alert: Option<Instant> = None;
        let mut last_sample = 0i64;
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
            // Arrêt non demandé depuis l'application (fenêtre du serveur fermée, crash…) : sauvegarde du monde figé,
            // avant toute relance automatique. Nos propres arrêts sont déjà sauvegardés par `ServerController::stop`.
            if prev_running && !snap.running && !expected && s.backup.on_stop {
                if let Some(e) = run_backup(&s, "arret-externe").await { warn_backup(&s, &mut last_backup_alert, "arrêt du serveur", &e).await; }
            }
            prev_running = snap.running;
            if let Some(e) = st.server.take_backup_warning() { warn_backup(&s, &mut last_backup_alert, "arrêt du serveur", &e).await; }

            let events = st.alerts.lock().await.evaluate(&s.alerts, &snap, expected);
            for e in &events {
                let _ = alerts::dispatch(&s.alerts, e).await;
                if matches!(e, alerts::Event::Crash) && s.auto_restart { let _ = st.server.start(&s).await; }
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
                        Action::Run(RuleAction::Start) => if !snap.running { scheduled_start(&st, &s).await },
                        Action::Run(RuleAction::Stop) => if snap.running { maintenance_stop(&st, &s).await },
                        Action::Run(RuleAction::Restart) => {
                            if snap.running { maintenance_restart(&st, &s).await } else { scheduled_start(&st, &s).await }
                        }
                    }
                }
                if let Some(th) = s.schedule.memory_restart_percent.filter(|_| s.schedule.enabled && snap.running) {
                    if snap.memory_percent >= th {
                        if let Some(api) = &api { let _ = api.announce("Mémoire élevée : redémarrage dans 1 minute.").await; }
                        tokio::time::sleep(Duration::from_secs(60)).await;
                        maintenance_restart(&st, &s).await;
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
                if used_gb >= limit as f64 && last_limit_restart.map_or(true, |t| t.elapsed() >= Duration::from_secs(1800)) {
                    last_limit_restart = Some(Instant::now());
                    let txt = format!("🟠 RAM du serveur : {used_gb:.1} Go (limite {limit:.1} Go){}", if s.performance.memory_limit_restart { " — redémarrage dans 1 minute." } else { "." });
                    let _ = alerts::dispatch_text(&s.alerts, &txt).await;
                    if s.performance.memory_limit_restart {
                        if let Some(api) = &api { let _ = api.announce("Mémoire élevée : redémarrage dans 1 minute.").await; }
                        tokio::time::sleep(Duration::from_secs(60)).await;
                        maintenance_restart(&st, &s).await;
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
                for n in now.difference(&known) { let _ = st.history.add_event(&PlayerEvent { t: ts, name: n.clone(), joined: true }); }
                for n in known.difference(&now) { let _ = st.history.add_event(&PlayerEvent { t: ts, name: n.clone(), joined: false }); }
                known = now;
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
