//! Boucle de fond : échantillonnage (5 s), alertes, backups planifiés, redémarrage auto après crash.
//! Émet l'événement `snapshot` vers le frontend à chaque tick.

use crate::state::AppState;
use palmanager_core::{alerts, backup, rest::RestClient};
use std::{sync::atomic::Ordering, time::{Duration, Instant}};
use tauri::{AppHandle, Emitter, Manager};

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_backup = Instant::now();
        let mut tick = tokio::time::interval(Duration::from_secs(5));
        loop {
            tick.tick().await;
            let st = app.state::<AppState>();
            let s = st.settings.read().await.clone();
            let api = RestClient::new(&s.rest).ok();

            let snap = st.monitor.lock().await.sample(st.server.pid(), api.as_ref()).await;
            let expected = st.expected_stop.load(Ordering::SeqCst);
            let events = st.alerts.lock().await.evaluate(&s.alerts, &snap, expected);
            for e in &events {
                let _ = alerts::dispatch(&s.alerts, e).await;
                if matches!(e, alerts::Event::Crash) && s.auto_restart { let _ = st.server.start(&s).await; }
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
