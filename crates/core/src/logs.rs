//! Lecture incrémentale du journal du serveur (`Pal/Saved/Logs/Pal.log`).

use crate::Result;
use serde::Serialize;
use std::{fs::File, io::{Read, Seek, SeekFrom}, path::Path};

#[derive(Debug, Serialize, PartialEq)]
pub struct LogChunk {
    pub lines: Vec<String>,
    /// Offset à repasser au prochain appel.
    pub offset: u64,
}

const MAX_INITIAL_BYTES: u64 = 64 * 1024;

/// `offset = None` : dernières lignes (≤ 64 Ko). Sinon, lit ce qui a été ajouté depuis `offset`
/// (repart du début si le fichier a été tronqué/rotaté). Un fichier absent donne un chunk vide.
pub fn read_from(path: &Path, offset: Option<u64>) -> Result<LogChunk> {
    let Ok(mut f) = File::open(path) else { return Ok(LogChunk { lines: vec![], offset: 0 }) };
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
    Ok(LogChunk { lines: text.lines().map(str::to_string).collect(), offset: len })
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
}
