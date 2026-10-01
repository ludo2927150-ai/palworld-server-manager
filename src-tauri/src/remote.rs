//! Branche l'API distante (crates/core/src/remote.rs) sur l'état de l'application.

use crate::{commands, state::AppState};
use palmanager_core::{
    backup::BackupInfo, history::{Sample, Session}, logs::LogChunk, monitor::Snapshot,
    remote::{self, Backend, Control}, Result,
};
use tauri::{AppHandle, Manager};

#[derive(Clone)]
pub struct TauriBackend(pub AppHandle);

impl Backend for TauriBackend {
    async fn snapshot(&self) -> Snapshot { commands::get_snapshot(self.0.state::<AppState>()).await.unwrap_or_default() }
    async fn control(&self, action: Control) -> Result<()> {
        let st = self.0.state::<AppState>();
        match action {
            Control::Start => commands::server_start(st).await,
            Control::Stop => commands::server_stop(st).await,
            Control::Restart => commands::server_restart(st).await,
        }
    }
    async fn history(&self, hours: i64) -> Vec<Sample> { commands::get_history(self.0.state::<AppState>(), hours).await.unwrap_or_default() }
    async fn sessions(&self, days: i64) -> Vec<Session> { commands::get_sessions(self.0.state::<AppState>(), days).await.unwrap_or_default() }
    async fn logs(&self, offset: Option<u64>) -> Result<LogChunk> { commands::read_logs(self.0.state::<AppState>(), offset).await }
    async fn backup_now(&self) -> Result<BackupInfo> { commands::backup_now(self.0.state::<AppState>()).await }
    async fn announce(&self, message: String) -> Result<()> { commands::announce(self.0.state::<AppState>(), message).await }
    async fn kick(&self, user_id: String) -> Result<()> { commands::kick_player(self.0.state::<AppState>(), user_id).await }
}

/// (Ré)applique les paramètres d'accès distant : arrête l'écoute en cours puis relance si activé.
pub async fn apply(app: &AppHandle) {
    let st = app.state::<AppState>();
    if let Some(h) = st.remote.lock().await.take() {
        h.abort();
        let _ = h.await; // attend la libération du port avant de rebinder
    }
    *st.remote_error.lock().await = None;
    let cfg = st.settings.read().await.remote.clone();
    if !cfg.enabled { return; }
    if cfg.token.len() < 16 {
        *st.remote_error.lock().await = Some("jeton d'accès trop court".into());
        return;
    }
    match remote::bind(cfg.port).await {
        Ok(listener) => {
            let backend = TauriBackend(app.clone());
            let h = tauri::async_runtime::spawn(async move { let _ = remote::run(listener, backend, cfg.token).await; });
            *st.remote.lock().await = Some(h);
        }
        Err(e) => *st.remote_error.lock().await = Some(format!("port {} indisponible : {e}", cfg.port)),
    }
}
