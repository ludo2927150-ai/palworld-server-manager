//! Analyse du journal du serveur : reconnaît les causes de problèmes courantes et propose quoi faire.
//! Ce sont des *causes probables* (motifs textuels), pas un diagnostic certain.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity { Critical, Warning, Info }

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub id: &'static str,
    pub severity: Severity,
    pub title: &'static str,
    pub advice: &'static str,
    /// Nombre de lignes correspondantes.
    pub count: usize,
    /// Dernière ligne correspondante (tronquée).
    pub sample: String,
}

struct Rule {
    id: &'static str,
    severity: Severity,
    title: &'static str,
    advice: &'static str,
    /// Une ligne correspond si elle contient l'un de ces motifs (minuscules).
    any: &'static [&'static str],
}

const RULES: &[Rule] = &[
    Rule { id: "oom", severity: Severity::Critical, title: "Mémoire insuffisante",
        advice: "Le serveur a manqué de RAM. Dans l'onglet Performance, fixez une limite de RAM avec redémarrage automatique ; désactivez les raids (bEnableInvaderEnemy) ; vérifiez la RAM libre du PC.",
        any: &["out of memory", "outofmemory", "failed to allocate", "bad_alloc", "not enough memory"] },
    Rule { id: "fatal", severity: Severity::Critical, title: "Plantage du moteur (erreur fatale)",
        advice: "Le jeu a rencontré une erreur fatale. Désactivez les mods récemment ajoutés (onglet Mods), mettez le serveur à jour, et restaurez une sauvegarde si le monde est endommagé.",
        any: &["exception_access_violation", "unhandled exception", "lowlevelfatalerror", "fatal error", "assertion failed", "a crash occurred"] },
    Rule { id: "disk", severity: Severity::Critical, title: "Disque plein",
        advice: "Plus d'espace disque : réduisez le nombre de sauvegardes conservées, supprimez d'anciennes archives ou libérez de la place.",
        any: &["no space left", "disk full", "not enough space on the disk", "enospc"] },
    Rule { id: "port", severity: Severity::Critical, title: "Port déjà utilisé",
        advice: "Un autre programme (ou une deuxième copie du serveur) utilise déjà ce port. Fermez l'autre serveur, ou changez PublicPort / RESTAPIPort dans la configuration.",
        any: &["failed to bind", "address already in use", "port is already in use", "address in use"] },
    Rule { id: "save", severity: Severity::Critical, title: "Problème d'écriture ou de lecture du monde",
        advice: "La sauvegarde du monde semble en échec ou corrompue. Restaurez la dernière bonne sauvegarde depuis l'onglet Sauvegardes (bouton « Revenir à celle-ci »).",
        any: &["failed to save", "save failed", "save data corrupt", "corrupted save", "failed to load world"] },
    Rule { id: "mod", severity: Severity::Warning, title: "Erreur liée à un mod",
        advice: "Un mod pose problème. Dans l'onglet Mods, désactivez-le (ou tous les mods) puis redémarrez pour confirmer.",
        any: &["mod error", "failed to load mod", "mod failed", "modloader", "ue4ss error"] },
    Rule { id: "rest", severity: Severity::Warning, title: "API REST indisponible",
        advice: "L'API REST n'a pas démarré. Vérifiez RESTAPIEnabled=True, le port et le mot de passe admin (onglet Diagnostic → « Corriger l'API REST »).",
        any: &["failed to start rest", "rest api failed", "rest api error"] },
    Rule { id: "version", severity: Severity::Warning, title: "Version du jeu différente",
        advice: "Des joueurs sont refusés à cause d'une version différente. Mettez le serveur à jour (bouton « Mettre à jour » du tableau de bord) ou demandez aux joueurs de mettre leur jeu à jour.",
        any: &["version mismatch", "incompatible version", "outdated client"] },
    Rule { id: "error", severity: Severity::Info, title: "Lignes d'erreur diverses",
        advice: "Des lignes d'erreur sont présentes sans cause connue. Si le serveur fonctionne normalement, elles sont souvent sans gravité.",
        any: &["[error]", "error:", " error "] },
];

/// Analyse des lignes de journal. Une ligne n'est comptée que pour la première règle qui la reconnaît
/// (les règles précises passent avant la règle générique « erreur »). Résultat trié : gravité puis fréquence.
pub fn analyze(lines: &[String]) -> Vec<Finding> {
    let mut found: Vec<Finding> = Vec::new();
    for line in lines {
        let low = line.to_lowercase();
        let Some(rule) = RULES.iter().find(|r| r.any.iter().any(|p| low.contains(p))) else { continue };
        let sample: String = line.trim().chars().take(200).collect();
        match found.iter_mut().find(|f| f.id == rule.id) {
            Some(f) => { f.count += 1; f.sample = sample; }
            None => found.push(Finding { id: rule.id, severity: rule.severity, title: rule.title, advice: rule.advice, count: 1, sample }),
        }
    }
    found.sort_by(|a, b| a.severity.cmp(&b.severity).then(b.count.cmp(&a.count)));
    found
}

/// Résumé d'une ligne pour les alertes : la cause la plus grave, ou `None` s'il n'y a rien de grave.
pub fn probable_cause(lines: &[String]) -> Option<&'static str> {
    analyze(lines).into_iter().find(|f| f.severity == Severity::Critical).map(|f| f.title)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(v: &[&str]) -> Vec<String> { v.iter().map(|s| s.to_string()).collect() }

    #[test]
    fn recognizes_causes_and_sorts_by_severity() {
        let f = analyze(&l(&[
            "[2026-10-01 10:00:00] [LOG] Server started",
            "LowLevelFatalError [File:Unknown] [Line: 1]",
            "[ERROR] something odd",
            "Failed to allocate 4294967296 bytes",
            "Failed to allocate 8 bytes",
            "[WARN] mod error in X",
        ]));
        let ids: Vec<_> = f.iter().map(|x| (x.id, x.count)).collect();
        // Critiques d'abord (oom ×2 avant fatal ×1), puis avertissement, puis info.
        assert_eq!(ids, [("oom", 2), ("fatal", 1), ("mod", 1), ("error", 1)]);
        assert_eq!(f[0].severity, Severity::Critical);
        assert!(f[0].advice.contains("Performance"));
    }

    #[test]
    fn precise_rule_wins_over_generic_error_and_normal_logs_are_ignored() {
        let f = analyze(&l(&["[ERROR] No space left on device", "REST accessed endpoint /v1/api/players OK", "Player Alice joined"]));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].id, "disk");
        assert!(analyze(&l(&["all good", "Running Palworld dedicated server on :8211"])).is_empty());
    }

    #[test]
    fn probable_cause_is_most_serious_only() {
        assert_eq!(probable_cause(&l(&["Address already in use"])), Some("Port déjà utilisé"));
        assert_eq!(probable_cause(&l(&["[ERROR] meh", "mod error"])), None); // pas de cause critique
        assert!(analyze(&[]).is_empty());
    }

    #[test]
    fn normal_startup_lines_do_not_trigger_anything() {
        let f = analyze(&l(&["Loading Level.sav ... done", "Game version is v1.0.5.102999", "client version check passed", "Saved world in 0.3s"]));
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn sample_is_truncated() {
        let long = format!("Fatal error {}", "x".repeat(500));
        assert!(analyze(&[long])[0].sample.chars().count() <= 200);
    }
}
