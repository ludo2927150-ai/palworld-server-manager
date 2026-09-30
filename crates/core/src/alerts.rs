//! Alertes : Discord (webhook) et push mobile (ntfy.sh). Le moteur détecte les transitions
//! (crash, mémoire, arrivée/départ de joueurs) à partir de `Snapshot`s successifs.

use crate::{monitor::Snapshot, settings::AlertSettings, Result};
use std::{collections::{HashMap, HashSet}, time::{Duration, Instant}};

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Crash,
    HighMemory { percent: f32 },
    PlayerJoined(String),
    PlayerLeft(String),
}

impl Event {
    fn key(&self) -> &'static str {
        match self { Event::Crash => "crash", Event::HighMemory { .. } => "mem", Event::PlayerJoined(_) => "join", Event::PlayerLeft(_) => "leave" }
    }
    pub fn message(&self) -> String {
        match self {
            Event::Crash => "🔴 Le serveur Palworld s'est arrêté de façon inattendue.".into(),
            Event::HighMemory { percent } => format!("🟠 Mémoire élevée : {percent:.0} %"),
            Event::PlayerJoined(n) => format!("🟢 {n} a rejoint le serveur"),
            Event::PlayerLeft(n) => format!("⚪ {n} a quitté le serveur"),
        }
    }
}

#[derive(Default)]
pub struct AlertEngine {
    was_running: bool,
    known_players: HashSet<String>,
    last_sent: HashMap<&'static str, Instant>,
}

impl AlertEngine {
    pub fn new() -> Self { Self::default() }

    /// Compare l'instantané à l'état précédent. `expected_stop` évite une fausse alerte de crash
    /// lors d'un arrêt demandé par l'utilisateur.
    pub fn evaluate(&mut self, cfg: &AlertSettings, snap: &Snapshot, expected_stop: bool) -> Vec<Event> {
        let mut ev = Vec::new();
        if self.was_running && !snap.running && !expected_stop && cfg.on_crash { ev.push(Event::Crash); }
        if snap.running {
            if let Some(th) = cfg.memory_threshold_percent {
                if snap.memory_percent >= th { ev.push(Event::HighMemory { percent: snap.memory_percent }); }
            }
        }
        let now: HashSet<String> = snap.players.iter().map(|p| p.name.clone()).collect();
        // Au premier instantané on ignore les joueurs déjà connectés (pas de rafale au démarrage de l'app).
        if self.was_running || !self.known_players.is_empty() {
            if cfg.on_player_join { ev.extend(now.difference(&self.known_players).cloned().map(Event::PlayerJoined)); }
            if cfg.on_player_leave { ev.extend(self.known_players.difference(&now).cloned().map(Event::PlayerLeft)); }
        }
        self.was_running = snap.running;
        self.known_players = now;
        // Anti-spam : cooldown par type d'événement (sauf connexions, toujours utiles).
        let cd = Duration::from_secs(cfg.cooldown_secs);
        ev.retain(|e| {
            if matches!(e, Event::PlayerJoined(_) | Event::PlayerLeft(_)) { return true; }
            let ok = self.last_sent.get(e.key()).map_or(true, |t| t.elapsed() >= cd);
            if ok { self.last_sent.insert(e.key(), Instant::now()); }
            ok
        });
        ev
    }
}

pub async fn dispatch(cfg: &AlertSettings, event: &Event) -> Result<()> {
    let http = reqwest::Client::builder().timeout(Duration::from_secs(10)).build()?;
    let msg = event.message();
    if let Some(url) = cfg.discord_webhook.as_deref().filter(|u| !u.is_empty()) {
        http.post(url).json(&serde_json::json!({ "content": msg })).send().await?.error_for_status()?;
    }
    if let Some(url) = cfg.ntfy_url.as_deref().filter(|u| !u.is_empty()) {
        http.post(url).header("Title", "Palworld").body(msg).send().await?.error_for_status()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Player;

    fn snap(running: bool, players: &[&str]) -> Snapshot {
        Snapshot { running, players: players.iter().map(|n| Player { name: n.to_string(), account_name: String::new(), player_id: String::new(), user_id: String::new(), level: 1, ping: 0.0 }).collect(), ..Default::default() }
    }

    #[test]
    fn detects_crash_and_joins() {
        let cfg = AlertSettings::default();
        let mut e = AlertEngine::new();
        assert!(e.evaluate(&cfg, &snap(true, &["A"]), false).is_empty()); // état initial
        assert_eq!(e.evaluate(&cfg, &snap(true, &["A", "B"]), false), vec![Event::PlayerJoined("B".into())]);
        assert_eq!(e.evaluate(&cfg, &snap(false, &[]), false), vec![Event::Crash]);
    }

    #[test]
    fn expected_stop_is_not_a_crash() {
        let cfg = AlertSettings::default();
        let mut e = AlertEngine::new();
        e.evaluate(&cfg, &snap(true, &[]), false);
        assert!(e.evaluate(&cfg, &snap(false, &[]), true).is_empty());
    }
}
