//! Journal d'audit : qui a fait quoi (application, mobile, Discord, automatismes), avec l'heure. Conservé 90 jours.

use serde::{Deserialize, Serialize};
use std::{io::Write, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditEntry { pub t: i64, pub who: String, pub action: String, pub detail: String }

pub struct AuditLog { path: PathBuf }

const RETENTION_SECS: i64 = 90 * 86_400;
const COMPACT_ABOVE_BYTES: u64 = 2 * 1024 * 1024;

fn now() -> i64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64) }

fn clean(s: &str, max: usize) -> String { s.chars().filter(|c| !c.is_control()).take(max).collect() }

impl AuditLog {
    pub fn new(path: PathBuf) -> Self { Self { path } }

    /// Ajoute une ligne. Au mieux : un échec d'écriture ne doit jamais bloquer l'action elle-même.
    pub fn record(&self, who: &str, action: &str, detail: &str) {
        self.record_at(now(), who, action, detail);
    }

    fn record_at(&self, t: i64, who: &str, action: &str, detail: &str) {
        let e = AuditEntry { t, who: clean(who, 80), action: clean(action, 80), detail: clean(detail, 300) };
        if let Some(dir) = self.path.parent() { let _ = std::fs::create_dir_all(dir); }
        if let (Ok(mut f), Ok(line)) = (std::fs::OpenOptions::new().create(true).append(true).open(&self.path), serde_json::to_string(&e)) {
            let _ = writeln!(f, "{line}");
        }
        if std::fs::metadata(&self.path).is_ok_and(|m| m.len() > COMPACT_ABOVE_BYTES) { self.compact(t); }
    }

    fn all(&self) -> Vec<AuditEntry> {
        std::fs::read_to_string(&self.path).map(|s| s.lines().filter_map(|l| serde_json::from_str(l).ok()).collect()).unwrap_or_default()
    }

    /// Retire ce qui dépasse la durée de conservation.
    fn compact(&self, now: i64) {
        let keep: Vec<String> = self.all().into_iter().filter(|e| e.t >= now - RETENTION_SECS).filter_map(|e| serde_json::to_string(&e).ok()).collect();
        let tmp = self.path.with_extension("jsonl.tmp");
        if std::fs::write(&tmp, keep.join("\n") + "\n").is_ok() { let _ = std::fs::rename(&tmp, &self.path); }
    }

    /// Les `limit` dernières entrées, la plus récente d'abord.
    pub fn recent(&self, limit: usize) -> Vec<AuditEntry> {
        let mut v = self.all();
        v.reverse();
        v.truncate(limit);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_orders_cleans_and_compacts() {
        let p = std::env::temp_dir().join(format!("pal-audit-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&p);
        let a = AuditLog::new(p.clone());
        a.record("app", "arrêt", "bouton\nArrêter");
        a.record("discord:42", "redémarrage", "");
        let r = a.recent(10);
        assert_eq!((r.len(), r[0].who.as_str(), r[1].detail.as_str()), (2, "discord:42", "boutonArrêter"));
        assert_eq!(a.recent(1).len(), 1);
        // entrée vieille de 100 jours : retirée au compactage
        a.record_at(now() - 100 * 86_400, "app", "vieux", "");
        a.compact(now());
        assert!(a.recent(10).iter().all(|e| e.action != "vieux") && a.recent(10).len() == 2);
        std::fs::remove_file(p).ok();
    }
}
