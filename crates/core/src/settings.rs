use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// Dossier d'installation (contient `PalServer.exe`).
    pub server_dir: PathBuf,
    /// Arguments de lancement, ex. `-useperfthreads -NoAsyncLoadingThread`.
    pub launch_args: Vec<String>,
    pub rest: RestSettings,
    pub backup: BackupSettings,
    pub alerts: AlertSettings,
    pub schedule: ScheduleSettings,
    /// Chemin de `steamcmd.exe` (installation et mises à jour du serveur).
    pub steamcmd_path: PathBuf,
    /// Relance automatique après un crash.
    pub auto_restart: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RestSettings {
    pub host: String,
    pub port: u16,
    pub admin_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BackupSettings {
    pub enabled: bool,
    pub interval_minutes: u64,
    pub retention: usize,
    pub destination: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScheduleSettings {
    pub enabled: bool,
    /// Heures de redémarrage quotidien, format `HH:MM` (heure locale).
    pub times: Vec<String>,
    /// Annonces en jeu, en minutes avant le redémarrage.
    pub announce_minutes: Vec<u32>,
    /// Redémarre (avec préavis d'1 minute) si la RAM du serveur dépasse ce pourcentage.
    pub memory_restart_percent: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AlertSettings {
    pub discord_webhook: Option<String>,
    /// Topic ntfy.sh (push mobile) : https://ntfy.sh/<topic>
    pub ntfy_url: Option<String>,
    pub on_crash: bool,
    pub on_player_join: bool,
    pub on_player_leave: bool,
    pub memory_threshold_percent: Option<f32>,
    pub cooldown_secs: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            server_dir: PathBuf::from(r"C:\palworld\PalServer"),
            launch_args: vec!["-useperfthreads".into(), "-NoAsyncLoadingThread".into(), "-UseMultithreadForDS".into()],
            rest: RestSettings::default(),
            backup: BackupSettings::default(),
            alerts: AlertSettings::default(),
            schedule: ScheduleSettings::default(),
            steamcmd_path: PathBuf::from("steamcmd.exe"),
            auto_restart: true,
        }
    }
}
impl Default for RestSettings {
    fn default() -> Self { Self { host: "127.0.0.1".into(), port: 8212, admin_password: String::new() } }
}
impl Default for BackupSettings {
    fn default() -> Self {
        Self { enabled: true, interval_minutes: 30, retention: 20, destination: PathBuf::from("backups") }
    }
}
impl Default for ScheduleSettings {
    fn default() -> Self {
        Self { enabled: false, times: vec!["04:00".into()], announce_minutes: vec![15, 5, 1], memory_restart_percent: None }
    }
}
impl Default for AlertSettings {
    fn default() -> Self {
        Self {
            discord_webhook: None, ntfy_url: None, on_crash: true, on_player_join: true,
            on_player_leave: false, memory_threshold_percent: Some(90.0), cooldown_secs: 300,
        }
    }
}

impl AppSettings {
    pub fn exe_path(&self) -> PathBuf { self.server_dir.join("PalServer.exe") }
    /// `<server>/Pal/Saved/Config/WindowsServer/PalWorldSettings.ini`
    pub fn world_settings_path(&self) -> PathBuf {
        self.server_dir.join("Pal/Saved/Config/WindowsServer/PalWorldSettings.ini")
    }
    /// `<server>/Pal/Saved/SaveGames`
    pub fn save_dir(&self) -> PathBuf { self.server_dir.join("Pal/Saved/SaveGames") }

    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(s) => Ok(serde_json::from_str(&s)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(p) = path.parent() { std::fs::create_dir_all(p)?; }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}
