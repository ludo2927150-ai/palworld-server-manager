//! Annonces automatiques en jeu : message de bienvenue et rappels réguliers.

use crate::settings::AnnouncementSettings;
use std::collections::HashMap;

pub const MAX_LEN: usize = 300;

/// Remplace `{nom}`, `{joueurs}` et `{max}` ; retire les caractères de contrôle et limite la longueur.
/// Le pseudo d'un joueur est une donnée non fiable : il n'est jamais interprété, seulement inséré comme texte.
pub fn render(template: &str, name: Option<&str>, online: usize, max: Option<u32>) -> String {
    let clean = |s: &str| s.chars().filter(|c| !c.is_control()).collect::<String>();
    let out = template
        .replace("{nom}", &clean(name.unwrap_or("")))
        .replace("{joueurs}", &online.to_string())
        .replace("{max}", &max.map_or("?".to_string(), |m| m.to_string()));
    clean(&out).trim().chars().take(MAX_LEN).collect()
}

#[derive(Default)]
pub struct Announcer {
    last: HashMap<String, i64>,
}

impl Announcer {
    pub fn new() -> Self { Self::default() }

    /// Textes à envoyer maintenant. Une règle ne se déclenche qu'après son intervalle complet (jamais au lancement de
    /// l'application) et, si `only_with_players`, seulement quand au moins un joueur est connecté.
    pub fn due(&mut self, cfg: &AnnouncementSettings, now: i64, online: usize, max: Option<u32>) -> Vec<String> {
        let mut out = Vec::new();
        self.last.retain(|id, _| cfg.rules.iter().any(|r| &r.id == id)); // règles supprimées
        if !cfg.enabled { return out; }
        for r in cfg.rules.iter().filter(|r| r.enabled && r.every_minutes >= 1 && !r.text.trim().is_empty()) {
            let last = *self.last.entry(r.id.clone()).or_insert(now);
            if now - last >= i64::from(r.every_minutes) * 60 {
                self.last.insert(r.id.clone(), now); // l'horloge avance même sans joueurs : pas de rafale au retour du premier joueur
                if !cfg.only_with_players || online > 0 { out.push(render(&r.text, None, online, max)); }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::AnnouncementRule;

    fn rule(id: &str, text: &str, every: u32) -> AnnouncementRule { AnnouncementRule { id: id.into(), text: text.into(), every_minutes: every, enabled: true } }
    fn cfg(rules: Vec<AnnouncementRule>) -> AnnouncementSettings { AnnouncementSettings { enabled: true, only_with_players: true, welcome: None, rules } }

    #[test]
    fn render_replaces_variables_and_cleans_untrusted_names() {
        assert_eq!(render("Bienvenue {nom} ! {joueurs}/{max}", Some("Alice"), 3, Some(32)), "Bienvenue Alice ! 3/32");
        assert_eq!(render("Salut {nom}", Some("Bo\u{7}b\n"), 1, None), "Salut Bob");
        assert_eq!(render("{max}", None, 0, None), "?");
        assert_eq!(render(&"x".repeat(1000), None, 0, None).chars().count(), MAX_LEN);
    }

    #[test]
    fn rules_fire_after_full_interval_not_at_launch() {
        let mut a = Announcer::new();
        let c = cfg(vec![rule("r1", "Pensez à vous hydrater", 10)]);
        assert!(a.due(&c, 1000, 2, Some(32)).is_empty());          // premier passage : l'horloge démarre
        assert!(a.due(&c, 1000 + 599, 2, Some(32)).is_empty());
        assert_eq!(a.due(&c, 1000 + 600, 2, Some(32)), ["Pensez à vous hydrater"]);
        assert!(a.due(&c, 1000 + 700, 2, Some(32)).is_empty());    // pas deux fois
        assert_eq!(a.due(&c, 1000 + 1200, 2, Some(32)).len(), 1);
    }

    #[test]
    fn nothing_is_sent_to_an_empty_server_and_no_burst_afterwards() {
        let mut a = Announcer::new();
        let c = cfg(vec![rule("r1", "Rappel", 5)]);
        a.due(&c, 0, 0, None);
        assert!(a.due(&c, 600, 0, None).is_empty());               // échéance passée, personne en ligne
        assert!(a.due(&c, 610, 1, None).is_empty());               // un joueur arrive : pas de rafale, on attend le prochain cycle
        assert_eq!(a.due(&c, 900, 1, None), ["Rappel"]);
    }

    #[test]
    fn disabled_rules_and_empty_texts_are_ignored() {
        let mut a = Announcer::new();
        let mut off = rule("r2", "x", 1); off.enabled = false;
        let c = cfg(vec![off, rule("r3", "   ", 1), rule("r4", "ok", 0)]);
        a.due(&c, 0, 5, None);
        assert!(a.due(&c, 10_000, 5, None).is_empty());
        let mut global_off = cfg(vec![rule("r1", "x", 1)]);
        global_off.enabled = false;
        assert!(a.due(&global_off, 10_000, 5, None).is_empty());
    }
}
