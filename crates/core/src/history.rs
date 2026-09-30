//! Historique local : échantillons de métriques (courbes 24 h) et connexions/déconnexions (sessions).
//! Stockage JSONL append-only dans un dossier de l'application ; purge à l'ouverture.

use crate::{monitor::Snapshot, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::{self, OpenOptions}, io::Write, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sample {
    /// Secondes Unix.
    pub t: i64,
    pub cpu: f32,
    pub mem_percent: f32,
    pub players: u32,
    pub fps: u32,
}

impl Sample {
    pub fn from_snapshot(t: i64, s: &Snapshot) -> Self {
        Self {
            t, cpu: s.cpu_percent, mem_percent: s.memory_percent, players: s.players.len() as u32,
            fps: s.metrics.as_ref().map_or(0, |m| m.serverfps),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerEvent {
    pub t: i64,
    pub name: String,
    pub joined: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Session {
    pub name: String,
    pub start: i64,
    /// `None` = toujours connecté.
    pub end: Option<i64>,
}

/// Apparie join/leave en sessions (les départs sans arrivée connue sont ignorés).
pub fn sessions(events: &[PlayerEvent]) -> Vec<Session> {
    let mut open: HashMap<&str, usize> = HashMap::new();
    let mut out: Vec<Session> = Vec::new();
    for e in events {
        if e.joined {
            open.insert(&e.name, out.len());
            out.push(Session { name: e.name.clone(), start: e.t, end: None });
        } else if let Some(i) = open.remove(e.name.as_str()) {
            out[i].end = Some(e.t);
        }
    }
    out
}

pub struct HistoryStore {
    samples: PathBuf,
    events: PathBuf,
}

const SAMPLE_RETENTION_SECS: i64 = 7 * 86_400;
const EVENT_RETENTION_SECS: i64 = 30 * 86_400;

fn read_lines<T: for<'de> Deserialize<'de>>(p: &PathBuf) -> Vec<T> {
    fs::read_to_string(p).map(|s| s.lines().filter_map(|l| serde_json::from_str(l).ok()).collect()).unwrap_or_default()
}

fn append<T: Serialize>(p: &PathBuf, v: &T) -> Result<()> {
    let mut f = OpenOptions::new().create(true).append(true).open(p)?;
    writeln!(f, "{}", serde_json::to_string(v)?)?;
    Ok(())
}

impl HistoryStore {
    /// Ouvre (et crée) le dossier, puis purge les entrées trop anciennes.
    pub fn open(dir: PathBuf, now: i64) -> Result<Self> {
        fs::create_dir_all(&dir)?;
        let st = Self { samples: dir.join("samples.jsonl"), events: dir.join("events.jsonl") };
        let keep: Vec<Sample> = read_lines::<Sample>(&st.samples).into_iter().filter(|s| s.t >= now - SAMPLE_RETENTION_SECS).collect();
        fs::write(&st.samples, keep.iter().map(|s| serde_json::to_string(s).unwrap() + "\n").collect::<String>())?;
        let keep: Vec<PlayerEvent> = read_lines::<PlayerEvent>(&st.events).into_iter().filter(|e| e.t >= now - EVENT_RETENTION_SECS).collect();
        fs::write(&st.events, keep.iter().map(|e| serde_json::to_string(e).unwrap() + "\n").collect::<String>())?;
        Ok(st)
    }

    pub fn add_sample(&self, s: &Sample) -> Result<()> { append(&self.samples, s) }
    pub fn add_event(&self, e: &PlayerEvent) -> Result<()> { append(&self.events, e) }

    /// Échantillons depuis `since`, réduits à au plus `max_points` (pas régulier) pour le graphique.
    pub fn samples_since(&self, since: i64, max_points: usize) -> Vec<Sample> {
        let all: Vec<Sample> = read_lines::<Sample>(&self.samples).into_iter().filter(|s| s.t >= since).collect();
        if all.len() <= max_points || max_points == 0 { return all; }
        let step = all.len().div_ceil(max_points);
        all.into_iter().step_by(step).collect()
    }

    pub fn events_since(&self, since: i64) -> Vec<PlayerEvent> {
        read_lines::<PlayerEvent>(&self.events).into_iter().filter(|e| e.t >= since).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf { std::env::temp_dir().join(format!("pal-hist-{name}-{}", std::process::id())) }

    #[test]
    fn stores_purges_and_downsamples() {
        let dir = tmp("a");
        let _ = fs::remove_dir_all(&dir);
        let st = HistoryStore::open(dir.clone(), 1_000_000).unwrap();
        for i in 0..100 { st.add_sample(&Sample { t: 1_000_000 - 100 + i, cpu: 1.0, mem_percent: 2.0, players: 0, fps: 60 }).unwrap(); }
        st.add_sample(&Sample { t: 1_000_000 - 8 * 86_400, cpu: 0.0, mem_percent: 0.0, players: 0, fps: 0 }).unwrap();
        let st = HistoryStore::open(dir.clone(), 1_000_000).unwrap(); // purge
        assert_eq!(st.samples_since(0, 1000).len(), 100);
        assert!(st.samples_since(0, 10).len() <= 10);
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn pairs_sessions() {
        let ev = |t, n: &str, j| PlayerEvent { t, name: n.into(), joined: j };
        let s = sessions(&[ev(1, "A", true), ev(2, "B", true), ev(5, "A", false), ev(9, "Z", false), ev(10, "A", true)]);
        assert_eq!(s.len(), 3);
        assert_eq!(s[0], Session { name: "A".into(), start: 1, end: Some(5) });
        assert_eq!(s[1].end, None);
        assert_eq!(s[2].end, None);
    }
}
