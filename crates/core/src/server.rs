//! Cycle de vie de `PalServer.exe`.

use crate::{rest::RestClient, settings::AppSettings, Error, Result};
use std::time::Duration;
use tokio::{process::{Child, Command}, sync::Mutex};

#[derive(Default)]
pub struct ServerController {
    child: Mutex<Option<Child>>,
}

impl ServerController {
    pub fn new() -> Self { Self::default() }

    /// `true` si le processus lancé par nous tourne encore.
    pub async fn is_running(&self) -> bool {
        let mut g = self.child.lock().await;
        match g.as_mut().map(|c| c.try_wait()) {
            Some(Ok(None)) => true,
            Some(_) => { *g = None; false }
            None => false,
        }
    }

    pub fn pid(&self) -> Option<u32> {
        self.child.try_lock().ok().and_then(|g| g.as_ref().and_then(|c| c.id()))
    }

    pub async fn start(&self, s: &AppSettings) -> Result<()> {
        if self.is_running().await { return Err(Error::Other("le serveur tourne déjà".into())); }
        let exe = s.exe_path();
        if !exe.exists() { return Err(Error::Other(format!("introuvable : {}", exe.display()))); }
        let child = Command::new(exe).args(&s.launch_args).current_dir(&s.server_dir).kill_on_drop(false).spawn()?;
        *self.child.lock().await = Some(child);
        Ok(())
    }

    /// Arrêt propre via l'API REST (sauvegarde + décompte), repli sur kill après `grace`.
    pub async fn stop(&self, s: &AppSettings, grace: Duration) -> Result<()> {
        if !self.is_running().await { return Ok(()); }
        if let Ok(api) = RestClient::new(&s.rest) {
            let _ = api.shutdown(10, "Arrêt du serveur dans 10 secondes").await;
        }
        let deadline = tokio::time::Instant::now() + grace;
        while tokio::time::Instant::now() < deadline {
            if !self.is_running().await { return Ok(()); }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        if let Some(mut c) = self.child.lock().await.take() { c.kill().await?; }
        Ok(())
    }

    pub async fn restart(&self, s: &AppSettings) -> Result<()> {
        self.stop(s, Duration::from_secs(60)).await?;
        self.start(s).await
    }
}
