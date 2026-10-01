//! Santé des sauvegardes : ancienneté de la dernière réussie et espace disque restant.

use crate::{backup::BackupInfo, settings::AlertSettings};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum Issue {
    NoRecentBackup { hours: i64 },
    NeverBackedUp,
    LowDisk { free_gb: f64, threshold_gb: u32 },
}

impl Issue {
    pub fn key(&self) -> &'static str {
        match self { Issue::NoRecentBackup { .. } | Issue::NeverBackedUp => "stale", Issue::LowDisk { .. } => "disk" }
    }
    pub fn message(&self) -> String {
        match self {
            Issue::NoRecentBackup { hours } => format!("⚠️ Aucune sauvegarde réussie depuis {hours} h alors que le serveur tourne."),
            Issue::NeverBackedUp => "⚠️ Aucune sauvegarde n'existe encore alors que le serveur tourne.".into(),
            Issue::LowDisk { free_gb, threshold_gb } => format!("⚠️ Espace disque faible pour les sauvegardes : {free_gb:.1} Go libres (seuil {threshold_gb} Go)."),
        }
    }
}

/// `backups` : toutes les sauvegardes du dossier ; `free_bytes` : espace libre du disque de destination (si connu).
/// `server_up_secs` évite de crier au démarrage : on laisse au moins le délai configuré s'écouler depuis le lancement du serveur.
pub fn evaluate(cfg: &AlertSettings, backups: &[BackupInfo], now: chrono::DateTime<chrono::Local>, running: bool, server_up_secs: i64, free_bytes: Option<u64>) -> Vec<Issue> {
    let mut v = Vec::new();
    if let (Some(h), true) = (cfg.stale_backup_hours, running) {
        let limit = h as i64 * 3600;
        if server_up_secs >= limit {
            match backups.iter().map(|b| b.created).max() {
                None => v.push(Issue::NeverBackedUp),
                Some(last) => {
                    let age = (now - last).num_seconds();
                    if age >= limit { v.push(Issue::NoRecentBackup { hours: (age + 1800) / 3600 }); }
                }
            }
        }
    }
    if let (Some(th), Some(free)) = (cfg.min_free_disk_gb, free_bytes) {
        let gb = free as f64 / 1e9;
        if gb < th as f64 { v.push(Issue::LowDisk { free_gb: gb, threshold_gb: th }); }
    }
    v
}

/// Espace libre du disque qui contient `path` (le point de montage le plus long qui en est le préfixe).
pub fn free_space(path: &Path) -> Option<u64> {
    let abs = std::fs::canonicalize(path).ok().or_else(|| path.ancestors().find_map(|a| std::fs::canonicalize(a).ok()))?;
    let disks = sysinfo::Disks::new_with_refreshed_list();
    disks.list().iter().filter(|d| abs.starts_with(d.mount_point())).max_by_key(|d| d.mount_point().as_os_str().len()).map(|d| d.available_space())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Local};

    fn bk(age_h: i64) -> BackupInfo {
        BackupInfo { file_name: "x.zip".into(), path: "x.zip".into(), size_bytes: 1, created: Local::now() - Duration::hours(age_h), protected: false }
    }

    #[test]
    fn stale_backup_detected_only_when_running_long_enough() {
        let cfg = AlertSettings::default(); // 6 h, 5 Go
        let now = Local::now();
        assert_eq!(evaluate(&cfg, &[bk(8), bk(9)], now, true, 20 * 3600, None), vec![Issue::NoRecentBackup { hours: 8 }]);
        assert!(evaluate(&cfg, &[bk(1)], now, true, 20 * 3600, None).is_empty());
        assert!(evaluate(&cfg, &[bk(8)], now, false, 20 * 3600, None).is_empty()); // serveur arrêté
        assert!(evaluate(&cfg, &[bk(8)], now, true, 600, None).is_empty()); // vient de démarrer
        assert_eq!(evaluate(&cfg, &[], now, true, 7 * 3600, None), vec![Issue::NeverBackedUp]);
    }

    #[test]
    fn low_disk_and_disabled_checks() {
        let mut cfg = AlertSettings::default();
        let now = Local::now();
        assert_eq!(evaluate(&cfg, &[bk(0)], now, true, 0, Some(2_000_000_000)).len(), 1);
        assert!(evaluate(&cfg, &[bk(0)], now, true, 0, Some(50_000_000_000)).is_empty());
        cfg.min_free_disk_gb = None; cfg.stale_backup_hours = None;
        assert!(evaluate(&cfg, &[], now, true, 99999, Some(0)).is_empty());
    }

    #[test]
    fn free_space_of_temp_dir_is_known_or_none_never_panics() {
        let _ = free_space(&std::env::temp_dir());
        let _ = free_space(Path::new("Z:/does/not/exist"));
    }
}
