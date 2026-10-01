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

/// Message d'accueil d'un joueur, par ordre de priorité : anniversaire ; message personnel ; retour après une longue absence ;
/// sinon le message de bienvenue général. `last_seen` : dernière visite connue (avant celle-ci) ; `today_md` : date du jour `MM-JJ`.
pub fn welcome_for(cfg: &AnnouncementSettings, user_id: &str, name: &str, last_seen: Option<i64>, now: i64, today_md: &str, online: usize, max: Option<u32>) -> Option<String> {
    let personal = cfg.personal.iter().find(|p| p.user_id == user_id);
    let tpl = if let Some(p) = personal.filter(|p| p.birthday.as_deref().is_some_and(|b| b.trim() == today_md)) {
        Some(p.text.clone().filter(|t| !t.trim().is_empty()).unwrap_or_else(|| "Joyeux anniversaire {nom} !".into()))
    } else if let Some(t) = personal.and_then(|p| p.text.clone()).filter(|t| !t.trim().is_empty()) {
        Some(t)
    } else if let (true, Some(seen), Some(t)) = (cfg.welcome_back_days > 0, last_seen, cfg.welcome_back_text.clone().filter(|t| !t.trim().is_empty())) {
        let days = (now - seen) / 86_400;
        (days >= cfg.welcome_back_days as i64).then(|| t.replace("{jours}", &days.to_string()))
    } else { None };
    let tpl = tpl.or_else(|| cfg.welcome.clone().filter(|t| !t.trim().is_empty()))?;
    Some(render(&tpl, Some(name), online, max))
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
    fn cfg(rules: Vec<AnnouncementRule>) -> AnnouncementSettings { AnnouncementSettings { enabled: true, only_with_players: true, welcome: None, rules, ..Default::default() } }

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

    #[test]
    fn personal_birthday_welcome_back_and_fallback() {
        use crate::settings::PersonalMessage;
        let cfg = AnnouncementSettings {
            enabled: true, welcome: Some("Bienvenue {nom}".into()),
            personal: vec![PersonalMessage { user_id: "steam_1".into(), name: "Alice".into(), text: Some("Salut la chef {nom}".into()), birthday: Some("10-01".into()) },
                           PersonalMessage { user_id: "steam_2".into(), name: "Bob".into(), text: None, birthday: Some("03-15".into()) }],
            welcome_back_days: 30, welcome_back_text: Some("Ça fait {jours} jours, {nom} !".into()), ..Default::default()
        };
        let day = 86_400;
        let w = |id: &str, name: &str, seen: Option<i64>, md: &str| welcome_for(&cfg, id, name, seen, 100 * day, md, 2, Some(32));
        assert_eq!(w("steam_1", "Alice", None, "10-01").unwrap(), "Salut la chef Alice"); // anniversaire avec texte perso
        assert_eq!(w("steam_2", "Bob", None, "03-15").unwrap(), "Joyeux anniversaire Bob !"); // anniversaire sans texte
        assert_eq!(w("steam_1", "Alice", None, "05-05").unwrap(), "Salut la chef Alice"); // texte perso
        assert_eq!(w("steam_3", "Zed", Some(100 * day - 45 * day), "05-05").unwrap(), "Ça fait 45 jours, Zed !");
        assert_eq!(w("steam_3", "Zed", Some(100 * day - 5 * day), "05-05").unwrap(), "Bienvenue Zed"); // absence courte : général
        assert_eq!(w("steam_2", "Bob", None, "05-05").unwrap(), "Bienvenue Bob");
        assert!(welcome_for(&AnnouncementSettings::default(), "x", "X", None, 0, "01-01", 1, None).is_none());
    }
}
