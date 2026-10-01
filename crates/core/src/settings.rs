use crate::{ini, Error, Result};
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
    /// Fermer la fenêtre la réduit dans la zone de notification au lieu de quitter.
    pub close_to_tray: bool,
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
    /// Second emplacement (autre disque, dossier synchronisé…) où chaque sauvegarde est aussi copiée.
    pub mirror_destination: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction { Start, Stop, Restart }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleRule {
    /// Heure locale, format `HH:MM`.
    pub time: String,
    pub action: RuleAction,
    /// Jours concernés : 0 = lundi … 6 = dimanche ; vide = tous les jours.
    #[serde(default)]
    pub days: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScheduleSettings {
    pub enabled: bool,
    /// Horaires programmés : démarrer, arrêter ou redémarrer le serveur.
    pub rules: Vec<ScheduleRule>,
    /// Annonces en jeu, en minutes avant un arrêt ou un redémarrage.
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
    /// Heure d'envoi du résumé quotidien (`HH:MM`, heure locale) ; `None` = désactivé.
    pub daily_summary_time: Option<String>,
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
            close_to_tray: false,
        }
    }
}
impl Default for RestSettings {
    fn default() -> Self { Self { host: "127.0.0.1".into(), port: 8212, admin_password: String::new() } }
}
impl Default for BackupSettings {
    fn default() -> Self {
        Self { enabled: true, interval_minutes: 30, retention: 20, destination: PathBuf::from("backups"), mirror_destination: None }
    }
}
impl Default for ScheduleSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            rules: vec![ScheduleRule { time: "04:00".into(), action: RuleAction::Restart, days: vec![] }],
            announce_minutes: vec![15, 5, 1],
            memory_restart_percent: None,
        }
    }
}
impl Default for AlertSettings {
    fn default() -> Self {
        Self {
            discord_webhook: None, ntfy_url: None, on_crash: true, on_player_join: true,
            on_player_leave: false, memory_threshold_percent: Some(90.0), cooldown_secs: 300, daily_summary_time: None,
        }
    }
}

impl AppSettings {
    pub fn exe_path(&self) -> PathBuf { self.server_dir.join("PalServer.exe") }
    /// `<server>/Pal/Saved/Config/WindowsServer/PalWorldSettings.ini`
    pub fn world_settings_path(&self) -> PathBuf {
        self.server_dir.join("Pal/Saved/Config/WindowsServer/PalWorldSettings.ini")
    }
    /// `<server>/DefaultPalWorldSettings.ini` : valeurs par défaut fournies avec le serveur.
    pub fn default_world_settings_path(&self) -> PathBuf { self.server_dir.join("DefaultPalWorldSettings.ini") }

    /// Options du monde. Sur une installation neuve `PalWorldSettings.ini` est vide (sans `OptionSettings`) :
    /// on repart alors de `DefaultPalWorldSettings.ini`. Renvoie aussi `true` si ce repli a été utilisé.
    pub fn load_world_options(&self) -> Result<(ini::Options, bool)> {
        let real = std::fs::read_to_string(self.world_settings_path()).map_err(Error::from).and_then(|c| ini::parse(&c));
        match real {
            Ok(o) => Ok((o, false)),
            Err(first) => match std::fs::read_to_string(self.default_world_settings_path()).map_err(Error::from).and_then(|c| ini::parse(&c)) {
                Ok(o) => Ok((o, true)),
                Err(_) => Err(Error::Other(format!(
                    "{first} — ni PalWorldSettings.ini ni DefaultPalWorldSettings.ini n'est exploitable dans {} (vérifiez le dossier du serveur dans « Application »)",
                    self.server_dir.display()
                ))),
            },
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_to_default_when_ini_is_empty() {
        let dir = std::env::temp_dir().join(format!("pal-world-{}", std::process::id()));
        let s = AppSettings { server_dir: dir.clone(), ..Default::default() };
        std::fs::create_dir_all(s.world_settings_path().parent().unwrap()).unwrap();
        std::fs::write(s.world_settings_path(), "[/Script/Pal.PalGameWorldSettings]\n").unwrap();
        assert!(s.load_world_options().is_err()); // pas de défaut disponible
        std::fs::write(s.default_world_settings_path(), "[/Script/Pal.PalGameWorldSettings]\nOptionSettings=(ExpRate=1.000000)\n").unwrap();
        let (o, from_default) = s.load_world_options().unwrap();
        assert!(from_default && o.len() == 1);
        std::fs::write(s.world_settings_path(), "[/Script/Pal.PalGameWorldSettings]\nOptionSettings=(ExpRate=2.000000)\n").unwrap();
        assert!(!s.load_world_options().unwrap().1);
        std::fs::remove_dir_all(dir).ok();
    }
}
