use palmanager_core::{alerts::AlertEngine, history::HistoryStore, schedule::Scheduler, monitor::{Monitor, Snapshot}, server::ServerController, settings::AppSettings};
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
        let history = HistoryStore::open(config_dir.join("history"), chrono::Utc::now().timestamp())?;
        Ok(Self {
            settings: RwLock::new({
                let mut s = AppSettings::load(&settings_path)?;
                s.resolve_backup_destination(&config_dir, &std::env::current_dir()?);
                s
            }),
            settings_path,
            server: ServerController::new(),
            monitor: Mutex::new(Monitor::new()),
            alerts: Mutex::new(AlertEngine::new()),
            history,
            scheduler: Mutex::new(Scheduler::new()),
            maintenance: AtomicBool::new(false),
            last_snapshot: RwLock::new(Snapshot::default()),
            expected_stop: AtomicBool::new(false),
        })
    }
}
