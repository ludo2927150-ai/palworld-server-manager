//! Résumé quotidien : texte court à partir de l'historique des dernières 24 h.

use crate::history::{sessions, PlayerEvent, Sample};
use std::collections::HashSet;

pub fn build(samples: &[Sample], events: &[PlayerEvent], now: i64) -> String {
    if samples.is_empty() {
        return "📊 Résumé des dernières 24 h : aucune donnée (le serveur n'a pas tourné).".into();
    }
    let n = samples.len() as f32;
    let avg = |f: fn(&Sample) -> f32| samples.iter().map(f).sum::<f32>() / n;
    let max = |f: fn(&Sample) -> f32| samples.iter().map(f).fold(0.0, f32::max);
    let peak = samples.iter().map(|s| s.players).max().unwrap_or(0);
    let unique: HashSet<&str> = events.iter().filter(|e| e.joined).map(|e| e.name.as_str()).collect();
    let longest = sessions(events).iter().map(|s| s.end.unwrap_or(now) - s.start).max().unwrap_or(0);
    // Un échantillon toutes les 30 s → temps de fonctionnement observé.
    let up_h = samples.len() as f32 * 30.0 / 3600.0;
    format!(
        "📊 Résumé des dernières 24 h\n• Serveur observé : {up_h:.1} h\n• Joueurs : {} différent(s), pic {peak}, plus longue session {} min\n• CPU : moyenne {:.0} %, max {:.0} %\n• Mémoire : moyenne {:.0} %, max {:.0} %",
        unique.len(), longest / 60, avg(|s| s.cpu), max(|s| s.cpu), avg(|s| s.mem_percent), max(|s| s.mem_percent),
    )
}

/// Rapport hebdomadaire (7 jours) : activité, joueurs les plus actifs, progression, nouveaux venus, disponibilité.
pub fn weekly(samples: &[Sample], events: &[PlayerEvent], players: &[crate::players::KnownPlayer], now: i64) -> String {
    let week_start = now - 7 * 86_400;
    let joins = events.iter().filter(|e| e.joined && e.t >= week_start).count();
    let unique: HashSet<&str> = events.iter().filter(|e| e.joined && e.t >= week_start).map(|e| e.name.as_str()).collect();
    let peak = samples.iter().filter(|s| s.t >= week_start).map(|s| s.players).max().unwrap_or(0);
    let observed = samples.iter().filter(|s| s.t >= week_start).count() as f32 * 30.0;
    let uptime = (observed / (7.0 * 86_400.0) * 100.0).min(100.0);
    // Temps de jeu par pseudo sur la semaine (les sessions à cheval sur le début de semaine sont tronquées).
    let mut play: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let evs: Vec<PlayerEvent> = events.iter().filter(|e| e.t >= week_start).cloned().collect();
    for s in sessions(&evs) { *play.entry(s.name.clone()).or_default() += s.end.unwrap_or(now) - s.start; }
    let mut top: Vec<(String, i64)> = play.into_iter().collect();
    top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let top_txt = if top.is_empty() { "personne".to_string() } else { top.iter().take(5).map(|(n, s)| format!("{n} ({} h {:02})", s / 3600, (s % 3600) / 60)).collect::<Vec<_>>().join(", ") };
    // Progression : niveau actuel moins le niveau au début de la semaine.
    let mut gains: Vec<(String, u32)> = players.iter().filter_map(|p| {
        let before = p.level_history.iter().rev().find(|(t, _)| *t <= week_start).map(|(_, l)| *l).or_else(|| p.level_history.first().map(|(_, l)| *l))?;
        (p.level > before && p.last_seen >= week_start).then(|| (p.name.clone(), p.level - before))
    }).collect();
    gains.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let gain_txt = if gains.is_empty() { "aucune".to_string() } else { gains.iter().take(3).map(|(n, g)| format!("{n} (+{g})")).collect::<Vec<_>>().join(", ") };
    let newcomers: Vec<&str> = players.iter().filter(|p| p.first_seen >= week_start).map(|p| p.name.as_str()).collect();
    format!(
        "📅 Rapport de la semaine\n• Connexions : {joins} ({} joueur(s) différent(s)), pic {peak} en même temps\n• Disponibilité observée : {uptime:.0} %\n• Les plus actifs : {top_txt}\n• Plus grosses progressions : {gain_txt}\n• Nouveaux joueurs : {}",
        unique.len(), if newcomers.is_empty() { "aucun".to_string() } else { newcomers.join(", ") },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_summary() {
        let s = |t, cpu, mem, p| Sample { t, cpu, mem_percent: mem, players: p, fps: 60 };
        let ev = |t, n: &str, j| PlayerEvent { t, name: n.into(), joined: j };
        let txt = build(&[s(0, 10.0, 50.0, 1), s(30, 30.0, 70.0, 3)], &[ev(0, "A", true), ev(10, "B", true), ev(3610, "A", false)], 4000);
        assert!(txt.contains("2 différent(s), pic 3"));
        assert!(txt.contains("CPU : moyenne 20 %, max 30 %"));
        assert!(txt.contains("plus longue session 66 min"));
        assert!(build(&[], &[], 0).contains("aucune donnée"));
    }

    #[test]
    fn weekly_report_summarises_activity() {
        use crate::players::KnownPlayer;
        let day = 86_400;
        let now = 10 * day;
        let s = |t, p| Sample { t, cpu: 1.0, mem_percent: 1.0, players: p, fps: 60 };
        let ev = |t, n: &str, j| PlayerEvent { t, name: n.into(), joined: j };
        let kp = |name: &str, first, last, lvl, hist: Vec<(i64, u32)>| KnownPlayer {
            user_id: name.into(), name: name.into(), previous_names: vec![], first_seen: first, last_seen: last, sessions: 1, total_secs: 0,
            player_id: String::new(), level: lvl, max_level: lvl, location: None, buildings: 0, level_history: hist,
        };
        let samples: Vec<Sample> = (0..2000).map(|i| s(4 * day + i * 30, if i % 100 == 0 { 4 } else { 1 })).collect();
        let events = [ev(5 * day, "Alice", true), ev(5 * day + 7200, "Alice", false), ev(6 * day, "Bob", true), ev(6 * day + 600, "Bob", false), ev(day, "Vieux", true)];
        let players = [kp("Alice", day, now, 25, vec![(day, 10), (5 * day, 15), (6 * day, 25)]), kp("Bob", 6 * day, now, 3, vec![(6 * day, 3)])];
        let txt = weekly(&samples, &events, &players, now);
        assert!(txt.contains("Connexions : 2 (2 joueur(s) différent(s)), pic 4"), "{txt}");
        assert!(txt.contains("Alice (2 h 00), Bob (0 h 10)"), "{txt}");
        assert!(txt.contains("Alice (+15)"), "{txt}"); // niveau 10 au début de la semaine -> 25
        assert!(txt.contains("Nouveaux joueurs : Bob"), "{txt}");
        assert!(weekly(&[], &[], &[], now).contains("Connexions : 0"));
    }
}
