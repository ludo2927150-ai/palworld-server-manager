use palmanager_core::{players::{BanBook, PlayerBook}, alerts::AlertEngine, history::HistoryStore, schedule::Scheduler, monitor::{Monitor, Snapshot}, server::ServerController, settings::AppSettings};
use std::{path::PathBuf, sync::atomic::AtomicBool};
use tauri::{AppHandle, Manager};
use tokio::sync::{Mutex, RwLock};

pub struct AppState {
    pub settings_path: PathBuf,
    pub settings: RwLock<AppSettings>,
    pub server: ServerController,
    pub monitor: Mutex<Monitor>,
    pub alerts: Mutex<AlertEngine>,
    pub history: HistoryStore,
    pub players: Mutex<PlayerBook>,
    pub bans: Mutex<BanBook>,
    /// Tâche d'écoute de l'accès distant (None = désactivé) et dernière erreur de démarrage.
    pub remote: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    pub remote_error: Mutex<Option<String>>,
    /// Clés valides de l'accès distant, partagées avec le serveur mobile (mises à jour à chaud).
    pub remote_creds: palmanager_core::remote::Credentials,
    /// Port sur lequel le serveur mobile écoute actuellement (pour ne pas le relancer inutilement).
    pub remote_port: Mutex<Option<u16>>,
    pub scheduler: Mutex<Scheduler>,
    /// Verrou : un seul redémarrage/mise à jour de maintenance à la fois.
    pub maintenance: AtomicBool,
    pub last_snapshot: RwLock<Snapshot>,
    /// Vrai entre une demande d'arrêt utilisateur et l'arrêt effectif (évite une fausse alerte de crash).
    pub expected_stop: AtomicBool,
}

impl AppState {
    pub fn load(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let config_dir = app.path().app_config_dir()?;
        let settings_path = config_dir.join("settings.json");
        crate::secrets::install();
        let history = HistoryStore::open(config_dir.join("history"), chrono::Utc::now().timestamp())?;
        Ok(Self {
            settings: RwLock::new({
                let mut s = AppSettings::load(&settings_path)?;
                s.resolve_backup_destination(&config_dir, &std::env::current_dir()?);
                // Migration : un fichier en clair est réécrit avec les secrets déplacés dans le Gestionnaire d'identifiants.
                let _ = s.save(&settings_path);
                s
            }),
            settings_path,
            server: ServerController::new(),
            monitor: Mutex::new(Monitor::new()),
            alerts: Mutex::new(AlertEngine::new()),
            history,
            players: Mutex::new(PlayerBook::open(config_dir.join("players.json"))),
            bans: Mutex::new(BanBook::open(config_dir.join("bans.json"))),
            remote: Mutex::new(None),
            remote_error: Mutex::new(None),
            remote_creds: Default::default(),
            remote_port: Mutex::new(None),
            scheduler: Mutex::new(Scheduler::new()),
            maintenance: AtomicBool::new(false),
            last_snapshot: RwLock::new(Snapshot::default()),
            expected_stop: AtomicBool::new(false),
        })
    }
}
