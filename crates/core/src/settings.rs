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
    pub performance: PerformanceSettings,
    pub announcements: AnnouncementSettings,
    pub access: AccessSettings,
    pub server_update: ServerUpdateSettings,
    pub discord_bot: DiscordBotSettings,
    pub remote: RemoteSettings,
    /// Chemin de `steamcmd.exe` (installation et mises à jour du serveur).
    pub steamcmd_path: PathBuf,
    /// Relance automatique après un crash.
    pub auto_restart: bool,
    /// Démarre le serveur automatiquement ~15 s après le lancement de l'application (utile avec « Lancer avec Windows »).
    pub start_server_on_launch: bool,
    /// Fermer la fenêtre la réduit dans la zone de notification au lieu de quitter.
    pub close_to_tray: bool,
    /// Assistant de premier lancement terminé ou ignoré.
    pub setup_done: bool,
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
    /// Sauvegarde systématique après chaque arrêt ou redémarrage du serveur (monde figé = copie cohérente).
    pub on_stop: bool,
}

/// Priorité CPU du processus serveur (la priorité « temps réel » est volontairement absente : elle peut figer Windows).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority { BelowNormal, #[default] Normal, AboveNormal, High }

/// Leviers de performance réellement disponibles pour un serveur natif Windows. La RAM ne s'« alloue » pas
/// (Palworld prend ce dont il a besoin) : on fixe une limite avec alerte ou redémarrage automatique.
/// Les options de threads au lancement se règlent via `launch_args`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct PerformanceSettings {
    pub priority: Priority,
    /// Cœurs logiques autorisés (0, 1, 2…) ; `None` = tous.
    pub cpu_cores: Option<Vec<u32>>,
    /// Limite de RAM du processus serveur, en Go ; `None` = aucune.
    pub memory_limit_gb: Option<f32>,
    /// Au-delà de la limite : redémarrer (avec annonce d'1 minute) au lieu d'alerter seulement.
    pub memory_limit_restart: bool,
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

/// Annonces en jeu : message de bienvenue et rappels réguliers. Variables : `{nom}` (bienvenue), `{joueurs}`, `{max}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AnnouncementSettings {
    pub enabled: bool,
    /// N'envoie les rappels que s'il y a au moins un joueur connecté.
    pub only_with_players: bool,
    pub welcome: Option<String>,
    pub rules: Vec<AnnouncementRule>,
}

impl Default for AnnouncementSettings {
    fn default() -> Self { Self { enabled: false, only_with_players: true, welcome: None, rules: Vec::new() } }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnnouncementRule {
    pub id: String,
    pub text: String,
    pub every_minutes: u32,
    pub enabled: bool,
}

/// Liste blanche appliquée par l'application : un joueur absent de la liste est expulsé dès sa connexion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct AccessSettings {
    pub whitelist_enabled: bool,
    pub allowed: Vec<AllowedPlayer>,
    /// Message affiché à l'expulsé.
    pub kick_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllowedPlayer {
    pub user_id: String,
    pub name: String,
}

/// Accès à distance (page web mobile servie par l'application). Désactivé par défaut.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RemoteSettings {
    pub enabled: bool,
    pub port: u16,
    /// Jeton secret du propriétaire (tous les droits), généré automatiquement à l'activation.
    pub token: String,
    /// Invitations : une clé par invité, avec des permissions limitées et une date d'expiration optionnelle.
    pub guests: Vec<Guest>,
}

/// Droits d'un invité. Volontairement limités : jamais les réglages, la configuration, la restauration ni le bannissement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Perm {
    /// Voir l'état du serveur (en ligne, CPU, RAM, FPS, nombre de joueurs).
    Status,
    /// Voir les pseudos des joueurs connectés et l'historique des sessions.
    Players,
    Logs,
    Charts,
    Start,
    Stop,
    Restart,
    Backup,
    Announce,
    Kick,
}

impl Perm {
    pub const ALL: [Perm; 10] = [Perm::Status, Perm::Players, Perm::Logs, Perm::Charts, Perm::Start, Perm::Stop, Perm::Restart, Perm::Backup, Perm::Announce, Perm::Kick];
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guest {
    /// Identifiant interne (pour révoquer) ; distinct de la clé.
    pub id: String,
    pub name: String,
    pub token: String,
    pub perms: Vec<Perm>,
    /// Secondes Unix ; `None` = n'expire pas.
    pub expires_at: Option<i64>,
    pub created_at: i64,
}

impl Default for RemoteSettings {
    fn default() -> Self { Self { enabled: false, port: 8765, token: String::new(), guests: Vec::new() } }
}

impl RemoteSettings {
    /// 32 caractères hexadécimaux issus du générateur aléatoire du système (128 bits).
    pub fn generate_token() -> Result<String> {
        let mut b = [0u8; 16];
        getrandom::getrandom(&mut b).map_err(|e| Error::Other(format!("génération du jeton : {e}")))?;
        Ok(b.iter().map(|x| format!("{x:02x}")).collect())
    }
    pub fn ensure_token(&mut self) -> Result<()> {
        if self.token.len() < 16 { self.token = Self::generate_token()?; }
        Ok(())
    }
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
    /// Notification Windows (toast) pour les mêmes événements, en plus de Discord/ntfy.
    pub desktop: bool,
    /// Alerte si aucune sauvegarde n'a réussi depuis ce nombre d'heures (serveur en marche) ; `None` = désactivé.
    pub stale_backup_hours: Option<u32>,
    /// Alerte si l'espace libre du disque de sauvegarde passe sous ce seuil (Go) ; `None` = désactivé.
    pub min_free_disk_gb: Option<u32>,
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
            performance: PerformanceSettings::default(),
            announcements: AnnouncementSettings::default(),
            access: AccessSettings::default(),
            server_update: ServerUpdateSettings::default(),
            discord_bot: DiscordBotSettings::default(),
            remote: RemoteSettings::default(),
            steamcmd_path: PathBuf::from("steamcmd.exe"),
            auto_restart: true,
            start_server_on_launch: false,
            close_to_tray: false,
            setup_done: false,
        }
    }
}
impl Default for RestSettings {
    fn default() -> Self { Self { host: "127.0.0.1".into(), port: 8212, admin_password: String::new() } }
}
impl Default for BackupSettings {
    fn default() -> Self {
        Self { enabled: true, interval_minutes: 30, retention: 10, destination: PathBuf::from("backups"), mirror_destination: None, on_stop: true }
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
/// Bot Discord (commandes slash). Connexion sortante uniquement ; seuls les identifiants Discord listés peuvent l'utiliser.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct DiscordBotSettings {
    pub enabled: bool,
    /// Jeton du bot (stocké dans le Gestionnaire d'identifiants sous Windows).
    pub bot_token: String,
    /// Identifiants Discord (numériques) autorisés. Liste vide = personne.
    pub allowed_user_ids: Vec<String>,
    /// Autorise aussi démarrer / arrêter / redémarrer / sauvegarder / annoncer (sinon lecture seule).
    pub allow_control: bool,
}

/// Mise à jour automatique du serveur Palworld (version Steam publique comparée à celle installée).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ServerUpdateSettings {
    pub enabled: bool,
    pub check_every_minutes: u64,
    /// Préavis donné aux joueurs avant l'arrêt (minutes). 0 = immédiat si le serveur est vide.
    pub warn_minutes: u64,
}
impl Default for ServerUpdateSettings {
    fn default() -> Self { Self { enabled: false, check_every_minutes: 60, warn_minutes: 5 } }
}

impl Default for AlertSettings {
    fn default() -> Self {
        Self {
            discord_webhook: None, ntfy_url: None, on_crash: true, on_player_join: true,
            on_player_leave: false, memory_threshold_percent: Some(90.0), cooldown_secs: 300, daily_summary_time: None, desktop: true,
            stale_backup_hours: Some(6), min_free_disk_gb: Some(5),
        }
    }
}

impl AppSettings {
    /// Chemin de `steamcmd.exe`, tolérant : guillemets/espaces superflus ignorés, et si le chemin saisi est un
    /// dossier (ex. `C:\\SteamCMD`) on y cherche `steamcmd.exe`.
    pub fn steamcmd_exe(&self) -> PathBuf {
        let raw = self.steamcmd_path.to_string_lossy();
        let p = PathBuf::from(raw.trim().trim_matches(|c| c == '"' || c == '\'').trim());
        if p.is_dir() { p.join("steamcmd.exe") } else { p }
    }

    /// Rend `backup.destination` absolu. Un chemin relatif dépendait du dossier de travail : `src-tauri` en développement
    /// (où le watcher de `tauri dev` relançait l'application à chaque archive écrite) et imprévisible une fois installé.
    /// Si d'anciennes sauvegardes existent à l'ancien emplacement relatif (`cwd/destination`), on les déplace.
    pub fn resolve_backup_destination(&mut self, base: &Path, cwd: &Path) {
        let d = self.backup.destination.clone();
        if d.is_absolute() { return; }
        let (old, new) = (cwd.join(&d), base.join(&d));
        if old.is_dir() && !new.exists() {
            if let Some(p) = new.parent() { let _ = std::fs::create_dir_all(p); }
            if std::fs::rename(&old, &new).is_err() { self.backup.destination = old; return; } // déplacement impossible : on garde l'ancien
        }
        self.backup.destination = new;
    }

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
            Ok(s) => {
                let mut v: Self = serde_json::from_str(&s)?;
                if let Some(store) = crate::secrets::global() { crate::secrets::reveal(store, &mut v); }
                Ok(v)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(p) = path.parent() { std::fs::create_dir_all(p)?; }
        let on_disk = match crate::secrets::global() {
            Some(store) => {
                if let Ok(prev) = std::fs::read_to_string(path).map_err(|_| ()).and_then(|j| serde_json::from_str::<Self>(&j).map_err(|_| ())) {
                    crate::secrets::prune_guests(store, &prev, self);
                }
                crate::secrets::protect(store, self)
            }
            None => self.clone(),
        };
        std::fs::write(path, serde_json::to_string_pretty(&on_disk)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_backup_destination_becomes_absolute_and_old_backups_move() {
        let root = std::env::temp_dir().join(format!("pal-dest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (cwd, base) = (root.join("cwd"), root.join("appdata"));
        std::fs::create_dir_all(cwd.join("backups")).unwrap();
        std::fs::write(cwd.join("backups/palworld-1.zip"), b"z").unwrap();
        let mut s = AppSettings::default(); // destination relative « backups »
        s.resolve_backup_destination(&base, &cwd);
        assert_eq!(s.backup.destination, base.join("backups"));
        assert!(base.join("backups/palworld-1.zip").exists() && !cwd.join("backups").exists());
        // Déjà absolu : inchangé. Rien à déplacer : simple résolution.
        let mut a = AppSettings::default();
        a.backup.destination = root.join("autre");
        a.resolve_backup_destination(&base, &cwd);
        assert_eq!(a.backup.destination, root.join("autre"));
        let mut b = AppSettings::default();
        b.resolve_backup_destination(&root.join("vide"), &cwd);
        assert_eq!(b.backup.destination, root.join("vide/backups"));
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn steamcmd_path_is_forgiving() {
        let dir = std::env::temp_dir().join(format!("pal-steam-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("steamcmd.exe"), b"").unwrap();
        let with = |p: String| AppSettings { steamcmd_path: PathBuf::from(p), ..Default::default() }.steamcmd_exe();
        let d = dir.to_string_lossy().to_string();
        assert_eq!(with(d.clone()), dir.join("steamcmd.exe"));                              // dossier
        assert_eq!(with(format!("\"{d}\"")), dir.join("steamcmd.exe"));                   // guillemets des deux côtés
        assert_eq!(with(format!("\"{d}")), dir.join("steamcmd.exe"));                      // guillemet d'ouverture seul
        assert_eq!(with(format!(" {}/steamcmd.exe ", d)), dir.join("steamcmd.exe"));        // fichier + espaces
        std::fs::remove_dir_all(dir).ok();
    }

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
