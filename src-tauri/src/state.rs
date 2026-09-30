use palmanager_core::{alerts::AlertEngine, monitor::{Monitor, Snapshot}, server::ServerController, settings::AppSettings};
use std::{path::PathBuf, sync::atomic::AtomicBool};
use tauri::{AppHandle, Manager};
use tokio::sync::{Mutex, RwLock};

pub struct AppState {
    pub settings_path: PathBuf,
    pub settings: RwLock<AppSettings>,
    pub server: ServerController,
    pub monitor: Mutex<Monitor>,
    pub alerts: Mutex<AlertEngine>,
    pub last_snapshot: RwLock<Snapshot>,
    /// Vrai entre une demande d'arrêt utilisateur et l'arrêt effectif (évite une fausse alerte de crash).
    pub expected_stop: AtomicBool,
}

impl AppState {
    pub fn load(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let settings_path = app.path().app_config_dir()?.join("settings.json");
        Ok(Self {
            settings: RwLock::new(AppSettings::load(&settings_path)?),
            settings_path,
            server: ServerController::new(),
            monitor: Mutex::new(Monitor::new()),
            alerts: Mutex::new(AlertEngine::new()),
            last_snapshot: RwLock::new(Snapshot::default()),
            expected_stop: AtomicBool::new(false),
        })
    }
}
