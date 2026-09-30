//! Boucle de fond : échantillonnage (5 s), alertes, planification des redémarrages, backups planifiés,
//! redémarrage auto après crash. Émet l'événement `snapshot` vers le frontend à chaque tick.

use crate::state::AppState;
use palmanager_core::{alerts, backup, rest::RestClient, schedule::Action, settings::AppSettings};
use std::{sync::atomic::Ordering, time::{Duration, Instant}};
use tauri::{AppHandle, Emitter, Manager};

/// Sauvegarde puis redémarre proprement (save REST → ZIP → arrêt → démarrage).
async fn maintenance_restart(st: &AppState, s: &AppSettings) {
    if st.maintenance.swap(true, Ordering::SeqCst) { return; }
    if let Ok(api) = RestClient::new(&s.rest) { let _ = api.save().await; }
    if backup::create(&s.save_dir(), &s.backup.destination).is_ok() {
        let _ = backup::rotate(&s.backup.destination, s.backup.retention);
    }
    st.expected_stop.store(true, Ordering::SeqCst);
    let _ = st.server.stop(s, Duration::from_secs(60)).await;
    st.expected_stop.store(false, Ordering::SeqCst);
    let _ = st.server.start(s).await;
    st.maintenance.store(false, Ordering::SeqCst);
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_backup = Instant::now();
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

            // Redémarrages planifiés (annonces en jeu puis restart) — uniquement si le serveur tourne.
            if snap.running && !maintenance {
                let now = chrono::Local::now().naive_local();
                let actions = st.scheduler.lock().await.tick(&s.schedule, now);
                for a in actions {
                    match a {
                        Action::Announce { minutes } => {
                            if let Some(api) = &api {
                                let _ = api.announce(&format!("Redémarrage du serveur dans {minutes} minute(s).")).await;
                            }
                        }
                        Action::Restart => maintenance_restart(&st, &s).await,
                    }
                }
                // Redémarrage préventif si la RAM dépasse le seuil (fuite mémoire) : préavis d'1 minute.
                if let Some(th) = s.schedule.memory_restart_percent.filter(|_| s.schedule.enabled) {
                    if snap.memory_percent >= th {
                        if let Some(api) = &api { let _ = api.announce("Mémoire élevée : redémarrage dans 1 minute.").await; }
                        tokio::time::sleep(Duration::from_secs(60)).await;
                        maintenance_restart(&st, &s).await;
                    }
                }
            }

            if s.backup.enabled && snap.running && last_backup.elapsed() >= Duration::from_secs(s.backup.interval_minutes.max(1) * 60) {
                if let Some(api) = &api { let _ = api.save().await; }
                if backup::create(&s.save_dir(), &s.backup.destination).is_ok() {
                    let _ = backup::rotate(&s.backup.destination, s.backup.retention);
                }
                last_backup = Instant::now();
            }

            *st.last_snapshot.write().await = snap.clone();
            let _ = app.emit("snapshot", snap);
        }
    });
}
