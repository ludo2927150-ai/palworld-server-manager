//! Boucle de fond : échantillonnage (5 s), alertes, planification des redémarrages, backups planifiés,
//! redémarrage auto après crash. Émet l'événement `snapshot` vers le frontend à chaque tick.

use crate::{commands::backup_everywhere, state::AppState};
use palmanager_core::{alerts, history::{PlayerEvent, Sample}, summary, rest::RestClient, schedule::Action, settings::{AppSettings, RuleAction}};
use std::{collections::HashSet, sync::atomic::Ordering, time::{Duration, Instant}};
use tauri::{AppHandle, Emitter, Manager};

/// Sauvegarde puis redémarre proprement (save REST → ZIP → arrêt → démarrage).
async fn maintenance_restart(st: &AppState, s: &AppSettings) {
    if st.maintenance.swap(true, Ordering::SeqCst) { return; }
    if let Ok(api) = RestClient::new(&s.rest) { let _ = api.save().await; }
    let _ = backup_everywhere(s);
    st.expected_stop.store(true, Ordering::SeqCst);
    let _ = st.server.stop(s, Duration::from_secs(60)).await;
    st.expected_stop.store(false, Ordering::SeqCst);
    let _ = st.server.start(s).await;
    st.maintenance.store(false, Ordering::SeqCst);
}

/// Arrêt planifié : sauvegarde puis arrêt propre. `expected_stop` reste vrai pour éviter une fausse alerte de
/// crash et un redémarrage automatique ; il est remis à faux au prochain démarrage.
async fn maintenance_stop(st: &AppState, s: &AppSettings) {
    if st.maintenance.swap(true, Ordering::SeqCst) { return; }
    if let Ok(api) = RestClient::new(&s.rest) { let _ = api.save().await; }
    let _ = backup_everywhere(s);
    st.expected_stop.store(true, Ordering::SeqCst);
    let _ = st.server.stop(s, Duration::from_secs(60)).await;
    st.maintenance.store(false, Ordering::SeqCst);
}

async fn scheduled_start(st: &AppState, s: &AppSettings) {
    st.expected_stop.store(false, Ordering::SeqCst);
    let _ = st.server.start(s).await;
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_backup = Instant::now();
        let mut daily = palmanager_core::schedule::DailyTrigger::new();
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

            if s.backup.enabled && snap.running && last_backup.elapsed() >= Duration::from_secs(s.backup.interval_minutes.max(1) * 60) {
                if let Some(api) = &api { let _ = api.save().await; }
                let _ = backup_everywhere(&s);
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
