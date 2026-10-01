//! Carnet des joueurs (historique, temps de jeu), liste des bannis et liste blanche.
//! Les identifiants viennent de l'API REST (`userId`, ex. `steam_7656…`). Tout est stocké localement en JSON.

use crate::{settings::AccessSettings, Result};
use serde::{Deserialize, Serialize};
use std::{collections::{HashMap, HashSet}, path::PathBuf};

/// Écart maximal compté entre deux observations : une interruption (application fermée, API muette)
/// ne gonfle pas le temps de jeu.
const MAX_GAP_SECS: i64 = 30;
const MAX_NAMES: usize = 5;
const MAX_LEVEL_POINTS: usize = 300;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnownPlayer {
    pub user_id: String,
    pub name: String,
    /// Anciens pseudos (les plus récents d'abord).
    pub previous_names: Vec<String>,
    pub first_seen: i64,
    pub last_seen: i64,
    pub sessions: u32,
    pub total_secs: u64,
    /// Identifiant de joueur du jeu (nom des fichiers `Players/<id>.sav`).
    #[serde(default)]
    pub player_id: String,
    #[serde(default)]
    pub level: u32,
    #[serde(default)]
    pub max_level: u32,
    /// Dernière position connue (coordonnées du jeu).
    #[serde(default)]
    pub location: Option<(f64, f64)>,
    #[serde(default)]
    pub buildings: u32,
    /// Évolution du niveau : (instant, niveau) à chaque changement (300 points au plus).
    #[serde(default)]
    pub level_history: Vec<(i64, u32)>,
}

/// Ce que l'API REST dit d'un joueur connecté.
#[derive(Debug, Clone, Default)]
pub struct Seen { pub user_id: String, pub name: String, pub player_id: String, pub level: u32, pub x: f64, pub y: f64, pub buildings: u32 }

impl Seen {
    pub fn from_rest(p: &crate::rest::Player) -> Self {
        Self { user_id: p.user_id.clone(), name: p.name.clone(), player_id: p.player_id.clone(), level: p.level, x: p.location_x, y: p.location_y, buildings: p.building_count }
    }
}

pub struct PlayerBook {
    path: PathBuf,
    players: HashMap<String, KnownPlayer>,
    online: HashSet<String>,
    last_tick: Option<i64>,
    dirty: bool,
}

impl PlayerBook {
    pub fn open(path: PathBuf) -> Self {
        let players: HashMap<String, KnownPlayer> = std::fs::read_to_string(&path).ok()
            .and_then(|t| serde_json::from_str::<Vec<KnownPlayer>>(&t).ok()).unwrap_or_default()
            .into_iter().map(|p| (p.user_id.clone(), p)).collect();
        Self { path, players, online: HashSet::new(), last_tick: None, dirty: false }
    }

    /// Met à jour le carnet avec les joueurs actuellement connectés (`(user_id, pseudo)`).
    /// Renvoie `(arrivées, départs)` sous forme d'identifiants.
    pub fn observe(&mut self, now: i64, current: &[Seen]) -> (Vec<String>, Vec<String>) {
        let gap = self.last_tick.map_or(0, |t| (now - t).clamp(0, MAX_GAP_SECS));
        let present: HashSet<&str> = current.iter().map(|c| c.user_id.as_str()).filter(|id| !id.is_empty()).collect();
        let (mut joined, mut left) = (Vec::new(), Vec::new());
        for c in current.iter().filter(|c| !c.user_id.is_empty()) {
            let (id, name) = (&c.user_id, &c.name);
            let p = self.players.entry(id.clone()).or_insert_with(|| KnownPlayer {
                user_id: id.clone(), name: name.clone(), previous_names: Vec::new(), first_seen: now, last_seen: now, sessions: 0, total_secs: 0,
                player_id: String::new(), level: 0, max_level: 0, location: None, buildings: 0, level_history: Vec::new(),
            });
            if !c.player_id.is_empty() { p.player_id = c.player_id.clone(); }
            if c.level > 0 && c.level != p.level {
                p.level = c.level;
                p.max_level = p.max_level.max(c.level);
                p.level_history.push((now, c.level));
                if p.level_history.len() > MAX_LEVEL_POINTS { p.level_history.remove(1); } // on garde toujours le premier point
            }
            if c.x != 0.0 || c.y != 0.0 { p.location = Some((c.x, c.y)); }
            if c.buildings > 0 || p.buildings > 0 { p.buildings = c.buildings; }
            if p.name != *name {
                let old = std::mem::replace(&mut p.name, name.clone());
                p.previous_names.retain(|n| n != &old && n != name);
                p.previous_names.insert(0, old);
                p.previous_names.truncate(MAX_NAMES);
            }
            if self.online.insert(id.clone()) { p.sessions += 1; joined.push(id.clone()); }
            else { p.total_secs += gap as u64; } // le temps ne compte que pour une présence continue
            p.last_seen = now;
        }
        for id in self.online.clone() {
            if !present.contains(id.as_str()) { self.online.remove(&id); left.push(id); }
        }
        self.last_tick = Some(now);
        self.dirty = true;
        (joined, left)
    }

    pub fn is_online(&self, id: &str) -> bool { self.online.contains(id) }

    /// Joueurs connus, du plus récemment vu au plus ancien.
    pub fn list(&self) -> Vec<KnownPlayer> {
        let mut v: Vec<_> = self.players.values().cloned().collect();
        v.sort_by(|a, b| b.last_seen.cmp(&a.last_seen).then(a.name.cmp(&b.name)));
        v
    }

    pub fn save_if_dirty(&mut self) -> Result<()> {
        if !self.dirty { return Ok(()); }
        if let Some(dir) = self.path.parent() { std::fs::create_dir_all(dir)?; }
        std::fs::write(&self.path, serde_json::to_string(&self.list())?)?;
        self.dirty = false;
        Ok(())
    }
}

// ───────────────────────── Bannis ─────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BanEntry {
    pub user_id: String,
    pub name: String,
    pub banned_at: i64,
    pub reason: Option<String>,
}

/// Bannissements faits depuis l'application (le serveur garde sa propre liste ; celle-ci sert à les retrouver et les lever).
pub struct BanBook {
    path: PathBuf,
    entries: Vec<BanEntry>,
}

impl BanBook {
    pub fn open(path: PathBuf) -> Self {
        let entries = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        Self { path, entries }
    }
    fn persist(&self) -> Result<()> {
        if let Some(dir) = self.path.parent() { std::fs::create_dir_all(dir)?; }
        std::fs::write(&self.path, serde_json::to_string(&self.entries)?)?;
        Ok(())
    }
    pub fn add(&mut self, e: BanEntry) -> Result<()> {
        self.entries.retain(|x| x.user_id != e.user_id);
        self.entries.insert(0, e);
        self.persist()
    }
    pub fn remove(&mut self, user_id: &str) -> Result<()> {
        self.entries.retain(|x| x.user_id != user_id);
        self.persist()
    }
    pub fn list(&self) -> Vec<BanEntry> { self.entries.clone() }
}

// ───────────────────────── Liste blanche ─────────────────────────

/// Faut-il expulser ce joueur ? Jamais si la liste blanche est désactivée, **ni si elle est vide** : activer une liste
/// vide expulserait tout le monde, propriétaire compris.
pub fn should_kick(cfg: &AccessSettings, user_id: &str) -> bool {
    cfg.whitelist_enabled && !cfg.allowed.is_empty() && !user_id.is_empty() && !cfg.allowed.iter().any(|a| a.user_id == user_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::AllowedPlayer;

    fn cur(v: &[(&str, &str)]) -> Vec<Seen> { v.iter().map(|(a, b)| Seen { user_id: a.to_string(), name: b.to_string(), ..Default::default() }).collect() }
    fn tmp(n: &str) -> PathBuf { std::env::temp_dir().join(format!("pal-pl-{n}-{}.json", std::process::id())) }

    #[test]
    fn tracks_sessions_playtime_and_name_changes() {
        let p = tmp("a");
        let _ = std::fs::remove_file(&p);
        let mut b = PlayerBook::open(p.clone());
        let (j, l) = b.observe(1000, &cur(&[("steam_1", "Alice")]));
        assert_eq!((j, l), (vec!["steam_1".to_string()], vec![]));
        b.observe(1005, &cur(&[("steam_1", "Alice")]));
        b.observe(1010, &cur(&[("steam_1", "AliceV2")])); // changement de pseudo
        let pl = &b.list()[0];
        assert_eq!((pl.sessions, pl.total_secs, pl.name.as_str(), pl.previous_names.clone()), (1, 10, "AliceV2", vec!["Alice".to_string()]));
        let (j2, l2) = b.observe(1015, &cur(&[]));
        assert_eq!((j2.len(), l2), (0, vec!["steam_1".to_string()]));
        assert!(!b.is_online("steam_1"));
        b.observe(1020, &cur(&[("steam_1", "AliceV2")])); // retour : nouvelle session, sans compter le temps hors ligne
        let pl = &b.list()[0];
        assert_eq!((pl.sessions, pl.total_secs, pl.first_seen), (2, 10, 1000));
    }

    #[test]
    fn details_track_level_history_position_and_buildings() {
        let mut b = PlayerBook::open(tmp("e"));
        let seen = |lvl, x, y, bld| vec![Seen { user_id: "steam_1".into(), name: "A".into(), player_id: "F8A7388F000000000000000000000000".into(), level: lvl, x, y, buildings: bld }];
        b.observe(100, &seen(5, 10.0, 20.0, 0));
        b.observe(105, &seen(5, 11.0, 21.0, 0)); // même niveau : pas de nouveau point
        b.observe(110, &seen(6, 0.0, 0.0, 3)); // position inconnue : on garde la précédente
        let p = &b.list()[0];
        assert_eq!(p.player_id, "F8A7388F000000000000000000000000");
        assert_eq!((p.level, p.max_level, p.buildings), (6, 6, 3));
        assert_eq!(p.level_history, vec![(100, 5), (110, 6)]);
        assert_eq!(p.location, Some((11.0, 21.0)));
        for i in 0..400 { b.observe(200 + i, &seen(7 + i as u32, 1.0, 1.0, 0)); }
        let p = &b.list()[0];
        assert!(p.level_history.len() <= MAX_LEVEL_POINTS && p.level_history[0] == (100, 5));
    }

    #[test]
    fn long_gaps_do_not_inflate_playtime_and_empty_ids_are_ignored() {
        let mut b = PlayerBook::open(tmp("b"));
        b.observe(0, &cur(&[("steam_1", "A"), ("", "Sans id")]));
        b.observe(10_000, &cur(&[("steam_1", "A")])); // l'application était fermée
        let pl = &b.list()[0];
        assert_eq!((pl.total_secs, b.list().len()), (MAX_GAP_SECS as u64, 1));
    }

    #[test]
    fn book_persists_and_reloads() {
        let p = tmp("c");
        let _ = std::fs::remove_file(&p);
        let mut b = PlayerBook::open(p.clone());
        b.observe(1, &cur(&[("steam_9", "Zed")]));
        b.save_if_dirty().unwrap();
        let again = PlayerBook::open(p.clone());
        assert_eq!(again.list()[0].user_id, "steam_9");
        assert!(!again.is_online("steam_9")); // l'état « en ligne » n'est jamais restauré du disque
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn ban_book_add_replace_remove() {
        let p = tmp("d");
        let _ = std::fs::remove_file(&p);
        let mut b = BanBook::open(p.clone());
        let e = |name: &str| BanEntry { user_id: "steam_1".into(), name: name.into(), banned_at: 5, reason: None };
        b.add(e("Vieux")).unwrap();
        b.add(e("Nouveau")).unwrap(); // même id : remplacé, pas dupliqué
        assert_eq!(b.list().len(), 1);
        assert_eq!(BanBook::open(p.clone()).list()[0].name, "Nouveau");
        b.remove("steam_1").unwrap();
        assert!(BanBook::open(p.clone()).list().is_empty());
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn whitelist_never_kicks_with_an_empty_list() {
        let allow = |ids: &[&str]| AccessSettings { whitelist_enabled: true, allowed: ids.iter().map(|i| AllowedPlayer { user_id: i.to_string(), name: String::new() }).collect(), kick_message: String::new() };
        assert!(!should_kick(&allow(&[]), "steam_1"));              // liste vide : jamais (sinon tout le monde dehors)
        assert!(!should_kick(&allow(&["steam_1"]), "steam_1"));     // autorisé
        assert!(should_kick(&allow(&["steam_1"]), "steam_2"));      // inconnu : dehors
        assert!(!should_kick(&allow(&["steam_1"]), ""));            // identifiant illisible : on ne tranche pas
        let mut off = allow(&["steam_1"]); off.whitelist_enabled = false;
        assert!(!should_kick(&off, "steam_2"));
    }
}
