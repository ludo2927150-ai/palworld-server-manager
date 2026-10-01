//! Auto-réparation : serveur gelé (processus vivant, API muette) et crashs en boucle. Logique pure (le temps est injecté).

use crate::settings::WatchdogSettings;

/// Délai de grâce après le démarrage du processus : l'API n'est pas encore prête.
const STARTUP_GRACE_SECS: i64 = 240;
/// Pas deux redémarrages « serveur gelé » à moins de 15 minutes d'intervalle.
const MIN_RESTART_GAP_SECS: i64 = 900;

#[derive(Default)]
pub struct Watchdog {
    rest_down_since: Option<i64>,
    /// L'API a répondu au moins une fois depuis le démarrage du processus (sans cela, une API désactivée passerait pour un gel).
    seen_ok: bool,
    last_hung_restart: Option<i64>,
    crashes: Vec<i64>,
}

impl Watchdog {
    pub fn new() -> Self { Self::default() }

    /// À appeler à chaque tick. `true` = le serveur est gelé, il faut le redémarrer maintenant.
    pub fn observe(&mut self, cfg: &WatchdogSettings, now: i64, running: bool, rest_ok: bool, up_secs: i64) -> bool {
        if !running { self.rest_down_since = None; self.seen_ok = false; return false; }
        if rest_ok { self.rest_down_since = None; self.seen_ok = true; return false; }
        if !cfg.enabled || !self.seen_ok || up_secs < STARTUP_GRACE_SECS { return false; }
        let since = *self.rest_down_since.get_or_insert(now);
        if now - since >= cfg.hung_minutes.max(1) as i64 * 60 && self.last_hung_restart.is_none_or(|t| now - t >= MIN_RESTART_GAP_SECS) {
            self.last_hung_restart = Some(now);
            self.rest_down_since = None;
            self.seen_ok = false;
            return true;
        }
        false
    }

    /// Enregistre un crash ; `true` = boucle de crashs détectée (la relance automatique doit s'arrêter).
    pub fn record_crash(&mut self, cfg: &WatchdogSettings, now: i64) -> bool {
        let window = cfg.crash_loop_window_minutes.max(1) as i64 * 60;
        self.crashes.retain(|t| now - t < window);
        self.crashes.push(now);
        cfg.enabled && self.crashes.len() >= cfg.crash_loop_max.max(2)
    }

    /// Après un démarrage voulu par l'utilisateur : on repart à zéro.
    pub fn reset_crashes(&mut self) { self.crashes.clear(); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hung_server_restarted_only_after_it_once_answered_and_long_enough() {
        let cfg = WatchdogSettings::default(); // 5 min
        let mut w = Watchdog::new();
        // API jamais joignable (désactivée) : jamais de redémarrage.
        for t in (0..3600).step_by(60) { assert!(!w.observe(&cfg, t, true, false, 10_000)); }
        // L'API répond, puis se tait.
        let mut w = Watchdog::new();
        assert!(!w.observe(&cfg, 0, true, true, 1000));
        assert!(!w.observe(&cfg, 10, true, false, 1010));
        assert!(!w.observe(&cfg, 200, true, false, 1200)); // < 5 min
        assert!(w.observe(&cfg, 311, true, false, 1311));
        // un redémarrage vient d'avoir lieu : pas de second avant 15 min, même si tout se répète
        assert!(!w.observe(&cfg, 320, true, true, 100));
        assert!(!w.observe(&cfg, 400, true, false, 300));
        assert!(!w.observe(&cfg, 800, true, false, 800));
    }

    #[test]
    fn startup_grace_and_disabled_and_recovery() {
        let cfg = WatchdogSettings::default();
        let mut w = Watchdog::new();
        w.observe(&cfg, 0, true, true, 1000);
        assert!(!w.observe(&cfg, 10, true, false, 100)); // vient de (re)démarrer
        assert!(!w.observe(&cfg, 1000, true, false, 100));
        let off = WatchdogSettings { enabled: false, ..cfg.clone() };
        let mut w = Watchdog::new();
        w.observe(&off, 0, true, true, 1000);
        assert!(!w.observe(&off, 10, true, false, 1010) && !w.observe(&off, 9999, true, false, 99999));
        // l'API revient : le compteur repart de zéro
        let mut w = Watchdog::new();
        w.observe(&cfg, 0, true, true, 1000);
        w.observe(&cfg, 10, true, false, 1010);
        w.observe(&cfg, 250, true, true, 1250);
        assert!(!w.observe(&cfg, 300, true, false, 1300));
        assert!(!w.observe(&cfg, 500, true, false, 1500));
    }

    #[test]
    fn crash_loop_detection() {
        let cfg = WatchdogSettings::default(); // 3 en 10 min
        let mut w = Watchdog::new();
        assert!(!w.record_crash(&cfg, 0));
        assert!(!w.record_crash(&cfg, 100));
        assert!(w.record_crash(&cfg, 200));
        w.reset_crashes();
        assert!(!w.record_crash(&cfg, 300));
        // crashs espacés : jamais une boucle
        let mut w = Watchdog::new();
        for i in 0..10 { assert!(!w.record_crash(&cfg, i * 1000)); }
    }
}
