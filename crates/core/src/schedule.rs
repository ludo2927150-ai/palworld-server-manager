//! Planificateur de redémarrages quotidiens avec annonces en jeu. Logique pure (l'heure est injectée)
//! pour être testable ; l'exécution des actions est faite par le superviseur.

use crate::settings::ScheduleSettings;
use chrono::{Duration, NaiveDateTime, NaiveTime};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Announce { minutes: u32 },
    Restart,
}

#[derive(Default)]
pub struct Scheduler {
    fired: HashSet<(NaiveDateTime, u32)>,
}

impl Scheduler {
    pub fn new() -> Self { Self::default() }

    /// À appeler à chaque tick. Chaque annonce/redémarrage n'est émis qu'une fois ; si plusieurs
    /// paliers sont déjà dépassés (app lancée tard), seul le plus proche du redémarrage est annoncé.
    pub fn tick(&mut self, cfg: &ScheduleSettings, now: NaiveDateTime) -> Vec<Action> {
        let mut out = Vec::new();
        if !cfg.enabled { return out; }
        self.fired.retain(|(t, _)| *t + Duration::days(1) > now);
        for time in cfg.times.iter().filter_map(|t| NaiveTime::parse_from_str(t.trim(), "%H:%M").ok()) {
            for date in [now.date(), now.date() + Duration::days(1)] {
                let target = date.and_time(time);
                let mut newly: Vec<u32> = Vec::new();
                for &m in &cfg.announce_minutes {
                    if now >= target - Duration::minutes(m as i64) && now < target && self.fired.insert((target, m.max(1))) {
                        newly.push(m);
                    }
                }
                if let Some(&m) = newly.iter().min() { out.push(Action::Announce { minutes: m }); }
                if now >= target && now < target + Duration::minutes(2) && self.fired.insert((target, 0)) {
                    out.push(Action::Restart);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(h: u32, m: u32, s: u32) -> NaiveDateTime { NaiveDate::from_ymd_opt(2026, 1, 1).unwrap().and_hms_opt(h, m, s).unwrap() }
    fn cfg() -> ScheduleSettings { ScheduleSettings { enabled: true, times: vec!["04:00".into()], announce_minutes: vec![15, 5, 1], memory_restart_percent: None } }

    #[test]
    fn announces_then_restarts_once() {
        let (mut s, c) = (Scheduler::new(), cfg());
        assert!(s.tick(&c, at(3, 30, 0)).is_empty());
        assert_eq!(s.tick(&c, at(3, 45, 0)), vec![Action::Announce { minutes: 15 }]);
        assert!(s.tick(&c, at(3, 45, 5)).is_empty());
        assert_eq!(s.tick(&c, at(3, 55, 0)), vec![Action::Announce { minutes: 5 }]);
        assert_eq!(s.tick(&c, at(3, 59, 1)), vec![Action::Announce { minutes: 1 }]);
        assert_eq!(s.tick(&c, at(4, 0, 0)), vec![Action::Restart]);
        assert!(s.tick(&c, at(4, 0, 5)).is_empty());
    }

    #[test]
    fn late_start_only_nearest_announce() {
        let (mut s, c) = (Scheduler::new(), cfg());
        assert_eq!(s.tick(&c, at(3, 58, 0)), vec![Action::Announce { minutes: 5 }]);
    }

    #[test]
    fn disabled_and_bad_times_do_nothing() {
        let mut s = Scheduler::new();
        assert!(s.tick(&ScheduleSettings { enabled: false, ..cfg() }, at(4, 0, 0)).is_empty());
        assert!(s.tick(&ScheduleSettings { times: vec!["nope".into()], ..cfg() }, at(4, 0, 0)).is_empty());
    }
}
