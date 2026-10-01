//! Cycle de vie de `PalServer.exe`. Le serveur est identifié par nom de processus : `PalServer.exe`
//! n'est qu'un lanceur qui engendre `PalServer-Win64-Shipping-Cmd.exe`, et un serveur lancé avant
//! l'application (ou en dehors d'elle) est ainsi « adopté » automatiquement.

use crate::{backup, rest::RestClient, settings::AppSettings, Error, Result};
use std::time::Duration;
use sysinfo::System;
use tokio::{process::{Child, Command}, sync::Mutex};

/// PID de tous les processus du serveur (lanceur + jeu).
pub fn find_server_pids() -> Vec<u32> {
    let mut sys = System::new();
    sys.refresh_processes();
    sys.processes().iter()
        .filter(|(_, p)| p.name().to_ascii_lowercase().starts_with("palserver"))
        .map(|(pid, _)| pid.as_u32()).collect()
}

fn kill_all() {
    let mut sys = System::new();
    sys.refresh_processes();
    for p in sys.processes().values().filter(|p| p.name().to_ascii_lowercase().starts_with("palserver")) { p.kill(); }
}

#[derive(Default)]
pub struct ServerController {
    child: Mutex<Option<Child>>,
    /// Dernière sauvegarde d'arrêt qui a échoué (récupérée une fois par le superviseur pour alerter).
    backup_warning: std::sync::Mutex<Option<String>>,
}

impl ServerController {
    pub fn new() -> Self { Self::default() }

    pub async fn is_running(&self) -> bool {
        let mut g = self.child.lock().await;
        if let Some(c) = g.as_mut() {
            if !matches!(c.try_wait(), Ok(Some(_))) { return true; }
            *g = None;
        }
        !find_server_pids().is_empty()
    }

    pub fn take_backup_warning(&self) -> Option<String> { self.backup_warning.lock().ok().and_then(|mut g| g.take()) }

    pub fn pid(&self) -> Option<u32> {
        self.child.try_lock().ok().and_then(|g| g.as_ref().and_then(|c| c.id())).or_else(|| find_server_pids().first().copied())
    }

    pub async fn start(&self, s: &AppSettings) -> Result<()> {
        if self.is_running().await { return Err(Error::Other("le serveur tourne déjà".into())); }
        let exe = s.exe_path();
        if !exe.exists() { return Err(Error::Other(format!("introuvable : {}", exe.display()))); }
        let child = Command::new(exe).args(&s.launch_args).current_dir(&s.server_dir).spawn()?;
        *self.child.lock().await = Some(child);
        Ok(())
    }

    /// Arrêt propre via l'API REST (sauvegarde + décompte), repli sur kill après `grace`.
    /// Une fois le serveur réellement arrêté (monde figé), une sauvegarde est faite si `backup.on_stop` est activé :
    /// tous les chemins (bouton, horaires, mise à jour, redémarrage) passent donc par ici.
    pub async fn stop(&self, s: &AppSettings, grace: Duration) -> Result<()> {
        if !self.is_running().await { return Ok(()); }
        if let Ok(api) = RestClient::new(&s.rest) {
            let _ = api.shutdown(10, "Arrêt du serveur dans 10 secondes").await;
        }
        let deadline = tokio::time::Instant::now() + grace;
        let mut stopped = false;
        while tokio::time::Instant::now() < deadline {
            if !self.is_running().await { stopped = true; break; }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        if !stopped {
            kill_all();
            *self.child.lock().await = None;
            tokio::time::sleep(Duration::from_secs(2)).await; // laisse Windows libérer les fichiers
        }
        if s.backup.on_stop {
            let s2 = s.clone();
            let label = if stopped { "arret" } else { "arret-force" };
            let res = tokio::task::spawn_blocking(move || backup::backup_all(&s2, Some(label))).await;
            let err = match res { Ok(Ok(_)) => None, Ok(Err(e)) => Some(e.to_string()), Err(e) => Some(e.to_string()) };
            if let (Some(e), Ok(mut g)) = (err, self.backup_warning.lock()) { *g = Some(e); }
        }
        Ok(())
    }

    pub async fn restart(&self, s: &AppSettings) -> Result<()> {
        self.stop(s, Duration::from_secs(60)).await?;
        self.start(s).await
    }
}
