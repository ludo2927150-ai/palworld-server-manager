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
}
