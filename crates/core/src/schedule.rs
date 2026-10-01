//! Planificateur de redémarrages quotidiens avec annonces en jeu. Logique pure (l'heure est injectée)
//! pour être testable ; l'exécution des actions est faite par le superviseur.

use crate::settings::{RuleAction, ScheduleSettings};
use chrono::{Datelike, Duration, NaiveDateTime, NaiveTime};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Prévenir les joueurs qu'un arrêt/redémarrage a lieu dans `minutes`.
    Announce { minutes: u32, action: RuleAction },
    /// Exécuter l'action programmée maintenant.
    Run(RuleAction),
}

/// Déclencheur quotidien (résumé) : renvoie `true` une seule fois par jour, à partir de l'heure choisie.
#[derive(Default)]
pub struct DailyTrigger {
    last: Option<chrono::NaiveDate>,
}

impl DailyTrigger {
    pub fn new() -> Self { Self::default() }
    pub fn due(&mut self, time: Option<&str>, now: NaiveDateTime) -> bool {
        let Some(t) = time.and_then(|t| NaiveTime::parse_from_str(t.trim(), "%H:%M").ok()) else { return false };
        if now.time() >= t && self.last != Some(now.date()) {
            self.last = Some(now.date());
            return true;
        }
        false
    }
}

#[derive(Default)]
pub struct Scheduler {
    /// (échéance, palier d'annonce — 0 = exécution, index de la règle)
    fired: HashSet<(NaiveDateTime, u32, usize)>,
}

impl Scheduler {
    pub fn new() -> Self { Self::default() }

    /// À appeler à chaque tick. Chaque annonce/exécution n'est émise qu'une fois ; si plusieurs paliers
    /// d'annonce sont déjà dépassés (app lancée tard), seul le plus proche est annoncé. Une exécution
    /// manquée de plus de 2 minutes (application fermée à ce moment) n'est pas rattrapée.
    pub fn tick(&mut self, cfg: &ScheduleSettings, now: NaiveDateTime) -> Vec<Action> {
        let mut out = Vec::new();
        if !cfg.enabled { return out; }
        self.fired.retain(|(t, _, _)| *t + Duration::days(1) > now);
        for (idx, rule) in cfg.rules.iter().enumerate() {
            let Ok(time) = NaiveTime::parse_from_str(rule.time.trim(), "%H:%M") else { continue };
            for date in [now.date(), now.date() + Duration::days(1)] {
                if !rule.days.is_empty() && !rule.days.contains(&(date.weekday().num_days_from_monday() as u8)) { continue; }
                let target = date.and_time(time);
                if rule.action != RuleAction::Start {
                    let mut newly: Vec<u32> = Vec::new();
                    for &m in &cfg.announce_minutes {
                        if now >= target - Duration::minutes(m as i64) && now < target && self.fired.insert((target, m.max(1), idx)) {
                            newly.push(m);
                        }
                    }
                    if let Some(&m) = newly.iter().min() { out.push(Action::Announce { minutes: m, action: rule.action }); }
                }
                if now >= target && now < target + Duration::minutes(2) && self.fired.insert((target, 0, idx)) {
                    out.push(Action::Run(rule.action));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ScheduleRule;
    use chrono::NaiveDate;

    // 2026-01-01 est un jeudi (days: 3).
    fn at(h: u32, m: u32, s: u32) -> NaiveDateTime { NaiveDate::from_ymd_opt(2026, 1, 1).unwrap().and_hms_opt(h, m, s).unwrap() }
    fn rule(time: &str, action: RuleAction, days: &[u8]) -> ScheduleRule { ScheduleRule { time: time.into(), action, days: days.to_vec() } }
    fn cfg(rules: Vec<ScheduleRule>) -> ScheduleSettings { ScheduleSettings { enabled: true, rules, announce_minutes: vec![15, 5, 1], memory_restart_percent: None } }
    use RuleAction::*;

    #[test]
    fn announces_then_restarts_once() {
        let (mut s, c) = (Scheduler::new(), cfg(vec![rule("04:00", Restart, &[])]));
        assert!(s.tick(&c, at(3, 30, 0)).is_empty());
        assert_eq!(s.tick(&c, at(3, 45, 0)), vec![Action::Announce { minutes: 15, action: Restart }]);
        assert!(s.tick(&c, at(3, 45, 5)).is_empty());
        assert_eq!(s.tick(&c, at(3, 55, 0)), vec![Action::Announce { minutes: 5, action: Restart }]);
        assert_eq!(s.tick(&c, at(3, 59, 1)), vec![Action::Announce { minutes: 1, action: Restart }]);
        assert_eq!(s.tick(&c, at(4, 0, 0)), vec![Action::Run(Restart)]);
        assert!(s.tick(&c, at(4, 0, 5)).is_empty());
    }

    #[test]
    fn start_has_no_announce_and_stop_does() {
        let c = cfg(vec![rule("07:00", Start, &[]), rule("23:00", Stop, &[])]);
        let mut s = Scheduler::new();
        assert!(s.tick(&c, at(6, 50, 0)).is_empty());
        assert_eq!(s.tick(&c, at(7, 0, 0)), vec![Action::Run(Start)]);
        assert_eq!(s.tick(&c, at(22, 56, 0)), vec![Action::Announce { minutes: 5, action: Stop }]);
        assert_eq!(s.tick(&c, at(23, 0, 1)), vec![Action::Run(Stop)]);
    }

    #[test]
    fn same_time_rules_do_not_collide() {
        let c = cfg(vec![rule("08:00", Stop, &[]), rule("08:00", Restart, &[])]);
        let mut s = Scheduler::new();
        let out = s.tick(&c, at(8, 0, 0));
        assert!(out.contains(&Action::Run(Stop)) && out.contains(&Action::Run(Restart)));
    }

    #[test]
    fn weekday_filter() {
        // Jeudi = 3 : une règle « lundi seulement » ne se déclenche pas, « jeudi » oui.
        let mut s = Scheduler::new();
        assert!(s.tick(&cfg(vec![rule("04:00", Restart, &[0])]), at(4, 0, 0)).is_empty());
        assert_eq!(s.tick(&cfg(vec![rule("04:00", Restart, &[3, 5])]), at(4, 0, 0)), vec![Action::Run(Restart)]);
    }

    #[test]
    fn daily_trigger_fires_once_per_day() {
        let mut d = DailyTrigger::new();
        assert!(!d.due(Some("20:00"), at(19, 59, 0)));
        assert!(d.due(Some("20:00"), at(20, 0, 5)));
        assert!(!d.due(Some("20:00"), at(20, 5, 0)));
        assert!(!d.due(None, at(21, 0, 0)) && !d.due(Some("pas une heure"), at(21, 0, 0)));
    }

    #[test]
    fn late_start_only_nearest_announce() {
        let (mut s, c) = (Scheduler::new(), cfg(vec![rule("04:00", Restart, &[])]));
        assert_eq!(s.tick(&c, at(3, 58, 0)), vec![Action::Announce { minutes: 5, action: Restart }]);
    }

    #[test]
    fn disabled_and_bad_times_do_nothing() {
        let mut s = Scheduler::new();
        let c = cfg(vec![rule("04:00", Restart, &[])]);
        assert!(s.tick(&ScheduleSettings { enabled: false, ..c }, at(4, 0, 0)).is_empty());
        assert!(s.tick(&cfg(vec![rule("nope", Restart, &[])]), at(4, 0, 0)).is_empty());
    }

    #[test]
    fn single_digit_hour_is_accepted() {
        let mut s = Scheduler::new();
        assert_eq!(s.tick(&cfg(vec![rule("4:00", Restart, &[])]), at(4, 0, 0)), vec![Action::Run(Restart)]);
    }
}
