//! Lecture incrémentale du journal du serveur. Le fichier est détecté automatiquement (le plus récent `.log` sous
//! `Pal/Saved`), car son nom varie selon la version du serveur ; un chemin peut aussi être imposé dans les réglages.

use crate::Result;
use serde::Serialize;
use std::{fs::File, io::{Read, Seek, SeekFrom}, path::{Path, PathBuf}};

#[derive(Debug, Serialize, PartialEq)]
pub struct LogChunk {
    pub lines: Vec<String>,
    /// Offset à repasser au prochain appel.
    pub offset: u64,
    /// Fichier lu (vide si aucun trouvé).
    pub source: String,
    /// Explication affichée quand aucun journal n'est trouvé.
    pub hint: Option<String>,
}

const MAX_INITIAL_BYTES: u64 = 64 * 1024;

/// `offset = None` : dernières lignes (≤ 64 Ko). Sinon, lit ce qui a été ajouté depuis `offset`
/// (repart du début si le fichier a été tronqué/rotaté). Un fichier absent donne un chunk vide.
pub fn read_from(path: &Path, offset: Option<u64>) -> Result<LogChunk> {
    let Ok(mut f) = File::open(path) else { return Ok(LogChunk { lines: vec![], offset: 0, source: String::new(), hint: None }) };
    let len = f.metadata()?.len();
    let start = match offset {
        None => len.saturating_sub(MAX_INITIAL_BYTES),
        Some(o) if o > len => 0,
        Some(o) => o,
    };
    f.seek(SeekFrom::Start(start))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    let mut text = String::from_utf8_lossy(&buf).into_owned();
    // Lecture initiale au milieu d'une ligne : on jette le fragment de début.
    if offset.is_none() && start > 0 { if let Some(i) = text.find('\n') { text.drain(..=i); } }
    Ok(LogChunk { lines: text.lines().map(str::to_string).collect(), offset: len, source: path.display().to_string(), hint: None })
}

/// Journal à lire : le chemin imposé s'il existe, sinon le `.log` le plus récemment modifié sous `Pal/Saved`.
pub fn locate(server_dir: &Path, custom: Option<&Path>, console: Option<&Path>) -> Option<PathBuf> {
    if let Some(c) = custom.filter(|c| !c.as_os_str().is_empty()) {
        return c.is_file().then(|| c.to_path_buf());
    }
    // Le dossier configuré peut être erroné alors que le serveur tourne ailleurs : on essaie aussi le dossier réel du processus.
    let running = crate::server::running_server_dir();
    let mut found: Vec<PathBuf> = [Some(server_dir.to_path_buf()), running].into_iter().flatten().filter_map(|root| newest_log_under(&root)).collect();
    // Sortie console enregistrée par l'application (serveur lancé depuis ici) : candidate comme les autres, la plus récente gagne.
    if let Some(c) = console.filter(|c| c.is_file()) { found.push(c.to_path_buf()); }
    found.into_iter().filter_map(|p| Some((p.metadata().ok()?.modified().ok()?, p))).max_by_key(|(m, _)| *m).map(|(_, p)| p)
}

fn newest_log_under(server_dir: &Path) -> Option<PathBuf> {
    walkdir::WalkDir::new(server_dir.join("Pal/Saved")).max_depth(4).into_iter().filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("log")))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.into_path())))
        .max_by_key(|(m, _)| *m).map(|(_, p)| p)
}

/// Comme `read_from`, sur le journal détecté ; sans fichier, explique où l'on a cherché et ce que contient le dossier.
pub fn read_located(server_dir: &Path, custom: Option<&Path>, console: Option<&Path>, offset: Option<u64>) -> Result<LogChunk> {
    match locate(server_dir, custom, console) {
        Some(p) => read_from(&p, offset),
        None => {
            let dir = server_dir.join("Pal/Saved/Logs");
            let names: Vec<String> = std::fs::read_dir(&dir).map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).take(12).collect()).unwrap_or_default();
            let hint = match custom.filter(|c| !c.as_os_str().is_empty()) {
                Some(c) => format!("Le fichier journal indiqué dans les réglages est introuvable : {}", c.display()),
                None if names.is_empty() => format!("Aucun journal disponible. Ce serveur n'écrit pas de fichier .log sous {} : l'application enregistre la console du serveur uniquement quand elle le démarre elle-même. Arrêtez le serveur puis relancez-le avec « Démarrer » depuis l'application.", server_dir.join("Pal/Saved").display()),
                None => format!("Aucun fichier .log sous {}. Contenu de {} : {}", server_dir.join("Pal/Saved").display(), dir.display(), names.join(", ")),
            };
            Ok(LogChunk { lines: vec![], offset: 0, source: String::new(), hint: Some(hint) })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn incremental_and_truncation() {
        let p = std::env::temp_dir().join(format!("pal-log-{}.log", std::process::id()));
        std::fs::write(&p, "a\nb\n").unwrap();
        let c = read_from(&p, None).unwrap();
        assert_eq!(c.lines, ["a", "b"]);
        std::fs::OpenOptions::new().append(true).open(&p).unwrap().write_all(b"c\n").unwrap();
        let c2 = read_from(&p, Some(c.offset)).unwrap();
        assert_eq!(c2.lines, ["c"]);
        std::fs::write(&p, "z\n").unwrap(); // rotation
        assert_eq!(read_from(&p, Some(c2.offset)).unwrap().lines, ["z"]);
        assert!(read_from(&p.with_extension("none"), None).unwrap().lines.is_empty());
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn locates_newest_log_and_explains_when_missing() {
        let root = std::env::temp_dir().join(format!("pal-loc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let c = read_located(&root, None, None, None).unwrap();
        assert!(c.lines.is_empty() && c.hint.is_some() && c.source.is_empty());
        let logs = root.join("Pal/Saved/Logs");
        std::fs::create_dir_all(&logs).unwrap();
        std::fs::write(logs.join("old.log"), "vieux\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30));
        std::fs::write(logs.join("PalServer-2026.LOG"), "[LOG] Kilz joined\n").unwrap();
        std::fs::write(root.join("Pal/Saved/notes.sav"), "x").unwrap();
        let c = read_located(&root, None, None, None).unwrap();
        assert_eq!(c.lines, ["[LOG] Kilz joined"]);
        assert!(c.source.ends_with("PalServer-2026.LOG") && c.hint.is_none());
        // chemin imposé
        let custom = logs.join("old.log");
        assert_eq!(read_located(&root, Some(&custom), None, None).unwrap().lines, ["vieux"]);
        assert!(read_located(&root, Some(&logs.join("nope.log")), None, None).unwrap().hint.unwrap().contains("introuvable"));
        std::fs::remove_dir_all(root).ok();
    }
}
