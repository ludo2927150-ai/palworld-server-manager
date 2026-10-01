//! Calendrier de saisons : un profil de configuration appliqué pendant une période (« XP ×2 du 1er au 7 »), puis retour
//! automatique à la configuration d'avant. Logique pure : la date est injectée.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeasonEvent {
    pub name: String,
    /// `AAAA-MM-JJ`, inclus.
    pub start: String,
    pub end: String,
    /// Profil de configuration appliqué pendant l'événement.
    pub profile: String,
    /// Annonce aux joueurs au début de l'événement (facultatif).
    #[serde(default)]
    pub announce: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SeasonState {
    /// Événement actuellement appliqué, et nom du profil de retour créé avant son application.
    pub applied: Option<String>,
    pub return_profile: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum Decision<'a> {
    Nothing,
    /// Appliquer l'événement (un profil de retour doit d'abord être enregistré).
    Start(&'a SeasonEvent),
    /// L'événement appliqué est terminé : revenir au profil de retour.
    End { event: String, return_profile: String },
}

fn day(s: &str) -> Option<NaiveDate> { NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok() }

pub fn is_active(e: &SeasonEvent, today: NaiveDate) -> bool {
    matches!((day(&e.start), day(&e.end)), (Some(a), Some(b)) if a <= today && today <= b)
}

/// Nom du profil de retour créé automatiquement avant un événement.
pub fn return_profile_name(event: &str) -> String { format!("avant-{}", event.chars().filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-').take(30).collect::<String>().trim()) }

pub fn decide<'a>(events: &'a [SeasonEvent], today: NaiveDate, state: &SeasonState) -> Decision<'a> {
    let active = events.iter().find(|e| is_active(e, today));
    match (&state.applied, active) {
        (None, Some(e)) => Decision::Start(e),
        (Some(applied), act) if act.map(|e| &e.name) != Some(applied) => match &state.return_profile {
            Some(rp) => Decision::End { event: applied.clone(), return_profile: rp.clone() },
            None => Decision::Nothing,
        },
        _ => Decision::Nothing,
    }
}

pub fn load_state(path: &std::path::Path) -> SeasonState {
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}
pub fn save_state(path: &std::path::Path, st: &SeasonState) {
    if let Some(d) = path.parent() { let _ = std::fs::create_dir_all(d); }
    if let Ok(j) = serde_json::to_string(st) { let _ = std::fs::write(path, j); }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(name: &str, a: &str, b: &str) -> SeasonEvent { SeasonEvent { name: name.into(), start: a.into(), end: b.into(), profile: "XP2".into(), announce: None } }
    fn d(s: &str) -> NaiveDate { day(s).unwrap() }

    #[test]
    fn start_then_end_with_return_profile() {
        let evs = [ev("Semaine XP", "2026-10-01", "2026-10-07")];
        let mut st = SeasonState::default();
        assert_eq!(decide(&evs, d("2026-09-30"), &st), Decision::Nothing);
        assert!(matches!(decide(&evs, d("2026-10-01"), &st), Decision::Start(e) if e.name == "Semaine XP"));
        assert!(matches!(decide(&evs, d("2026-10-07"), &st), Decision::Start(_))); // fin incluse
        st = SeasonState { applied: Some("Semaine XP".into()), return_profile: Some("avant-Semaine XP".into()) };
        assert_eq!(decide(&evs, d("2026-10-05"), &st), Decision::Nothing);
        assert_eq!(decide(&evs, d("2026-10-08"), &st), Decision::End { event: "Semaine XP".into(), return_profile: "avant-Semaine XP".into() });
    }

    #[test]
    fn invalid_dates_never_activate_and_events_can_follow_each_other() {
        assert!(!is_active(&ev("x", "pas une date", "2026-10-07"), d("2026-10-01")));
        let evs = [ev("A", "2026-10-01", "2026-10-03"), ev("B", "2026-10-04", "2026-10-06")];
        let st = SeasonState { applied: Some("A".into()), return_profile: Some("avant-A".into()) };
        // A se termine et B commence le même jour : on revient d'abord à l'état d'avant, puis B démarrera au tick suivant
        assert!(matches!(decide(&evs, d("2026-10-04"), &st), Decision::End { .. }));
        assert!(matches!(decide(&evs, d("2026-10-04"), &SeasonState::default()), Decision::Start(e) if e.name == "B"));
        assert_eq!(return_profile_name("Raid ../x"), "avant-Raid x");
    }
}
