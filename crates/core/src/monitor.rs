//! Métriques système du processus serveur + joueurs (via REST).

use crate::rest::{Metrics, Player, RestClient};
use serde::Serialize;
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};

#[derive(Debug, Clone, Serialize, Default)]
pub struct Snapshot {
    pub running: bool,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub memory_percent: f32,
    pub total_memory_bytes: u64,
    pub metrics: Option<Metrics>,
    pub players: Vec<Player>,
}

pub struct Monitor {
    sys: System,
}

impl Default for Monitor {
    fn default() -> Self { Self::new() }
}

impl Monitor {
    pub fn new() -> Self {
        Self { sys: System::new_with_specifics(RefreshKind::new().with_memory(sysinfo::MemoryRefreshKind::everything()).with_processes(ProcessRefreshKind::everything())) }
    }

    /// Cherche `PalServer-Win64-Shipping-Cmd.exe` (le vrai processus de jeu, enfant de PalServer.exe) ;
    /// à défaut le PID fourni. Le CPU exige deux rafraîchissements espacés (>= 200 ms).
    pub async fn sample(&mut self, pid_hint: Option<u32>, api: Option<&RestClient>) -> Snapshot {
        self.sys.refresh_memory();
        self.sys.refresh_processes();
        let total = self.sys.total_memory();
        let proc = self.sys.processes().values()
            .find(|p| p.name().to_ascii_lowercase().starts_with("palserver-win64-shipping"))
            .or_else(|| pid_hint.and_then(|p| self.sys.process(Pid::from_u32(p))));
        let mut snap = Snapshot { total_memory_bytes: total, ..Default::default() };
        if let Some(p) = proc {
            snap.running = true;
            snap.cpu_percent = p.cpu_usage() / self.sys.cpus().len().max(1) as f32;
            snap.memory_bytes = p.memory();
            snap.memory_percent = if total > 0 { p.memory() as f32 / total as f32 * 100.0 } else { 0.0 };
            if let Some(api) = api {
                snap.metrics = api.metrics().await.ok();
                snap.players = api.players().await.unwrap_or_default();
            }
        }
        snap
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub cpu_cores: usize,
    pub total_memory_bytes: u64,
}

pub fn system_info() -> SystemInfo {
    let mut sys = System::new();
    sys.refresh_memory();
    SystemInfo {
        cpu_cores: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
        total_memory_bytes: sys.total_memory(),
    }
}
