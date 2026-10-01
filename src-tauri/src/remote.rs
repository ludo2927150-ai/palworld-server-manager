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

/// (Ré)applique les paramètres d'accès distant. Les clés sont toujours rafraîchies à chaud (invitation créée ou
/// révoquée = effet immédiat, sans couper les connexions) ; l'écoute n'est relancée que si l'état ou le port change.
pub async fn apply(app: &AppHandle) {
    apply_discord(app).await;
    apply_remote(app).await;
}

/// (Re)lance ou arrête le bot Discord selon les réglages ; inchangé si les réglages n'ont pas bougé.
async fn apply_discord(app: &AppHandle) {
    let st = app.state::<AppState>();
    let cfg = st.settings.read().await.discord_bot.clone();
    let wanted = cfg.enabled && !cfg.bot_token.trim().is_empty();
    let mut cur = st.discord.lock().await;
    if let Some((_, running_cfg)) = cur.as_ref() {
        if wanted && *running_cfg == cfg { return; }
    }
    if let Some((h, _)) = cur.take() { h.abort(); }
    if wanted {
        let h = tauri::async_runtime::spawn(palmanager_core::discord::run(TauriBackend(app.clone()), cfg.clone()));
        *cur = Some((h, cfg));
    }
}

async fn apply_remote(app: &AppHandle) {
    let st = app.state::<AppState>();
    let cfg = st.settings.read().await.remote.clone();
    st.remote_creds.set(if cfg.enabled { remote::credentials(&cfg) } else { Vec::new() });

    let listening = st.remote.lock().await.is_some();
    let bound_port = *st.remote_port.lock().await;
    if cfg.enabled && listening && bound_port == Some(cfg.port) { return; }

    if let Some(h) = st.remote.lock().await.take() {
        h.abort();
        let _ = h.await; // attend la libération du port avant de rebinder
    }
    *st.remote_port.lock().await = None;
    *st.remote_error.lock().await = None;
    if !cfg.enabled { return; }
    if cfg.token.len() < 16 {
        *st.remote_error.lock().await = Some("clé d'accès trop courte".into());
        return;
    }
    match remote::bind(cfg.port).await {
        Ok(listener) => {
            let (backend, creds) = (TauriBackend(app.clone()), st.remote_creds.clone());
            let h = tauri::async_runtime::spawn(async move { let _ = remote::run(listener, backend, creds).await; });
            *st.remote.lock().await = Some(h);
            *st.remote_port.lock().await = Some(cfg.port);
        }
        Err(e) => *st.remote_error.lock().await = Some(format!("port {} indisponible : {e}", cfg.port)),
    }
}
