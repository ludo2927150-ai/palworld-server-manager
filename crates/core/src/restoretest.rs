//! Test de restauration : la dernière sauvegarde est relue, extraite dans un dossier temporaire et contrôlée
//! (fichiers du monde présents et non vides), puis le dossier temporaire est supprimé. Ne touche jamais au monde réel.

use crate::{backup, Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RestoreTestState { pub t: i64, pub ok: bool, pub detail: String }

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RestoreTestReport { pub archive: String, pub files: usize, pub bytes: u64, pub players: usize }

pub fn run(archive: &Path) -> Result<RestoreTestReport> {
    let rep = backup::verify(archive)?;
    let tmp = std::env::temp_dir().join(format!("palmanager-restoretest-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis())));
    let result = (|| -> Result<RestoreTestReport> {
        backup::restore_into(archive, &tmp)?;
        let (mut level, mut meta, mut players) = (false, false, 0usize);
        for e in walkdir::WalkDir::new(&tmp).into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()) {
            let name = e.file_name().to_string_lossy().to_string();
            let size = e.metadata().map(|m| m.len()).unwrap_or(0);
            match name.as_str() {
                "Level.sav" => level = size > 0,
                "LevelMeta.sav" => meta = size > 0,
                n if n.ends_with(".sav") && e.path().parent().and_then(|p| p.file_name()).is_some_and(|p| p == "Players") && size > 0 => players += 1,
                _ => {}
            }
        }
        if !level { return Err(Error::Other("Level.sav absent ou vide : cette sauvegarde ne contient pas de monde utilisable".into())); }
        if !meta { return Err(Error::Other("LevelMeta.sav absent ou vide".into())); }
        Ok(RestoreTestReport { archive: archive.file_name().map(|n| n.to_string_lossy().into()).unwrap_or_default(), files: rep.files, bytes: rep.bytes, players })
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

pub fn load_state(path: &Path) -> RestoreTestState { std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default() }
pub fn save_state(path: &Path, st: &RestoreTestState) {
    if let Some(d) = path.parent() { let _ = std::fs::create_dir_all(d); }
    if let Ok(j) = serde_json::to_string(st) { let _ = std::fs::write(path, j); }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_world(root: &Path, level: &[u8], meta: &[u8]) {
        let w = root.join("SaveGames/0/WORLD");
        std::fs::create_dir_all(w.join("Players")).unwrap();
        std::fs::write(w.join("Level.sav"), level).unwrap();
        std::fs::write(w.join("LevelMeta.sav"), meta).unwrap();
        std::fs::write(w.join("Players/F8A7388F000000000000000000000000.sav"), b"p").unwrap();
    }

    #[test]
    fn good_world_passes_and_temp_is_cleaned_bad_worlds_fail() {
        let root = std::env::temp_dir().join(format!("pal-rt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        make_world(&root, b"level-data", b"meta");
        let a = backup::create(&root.join("SaveGames"), &root.join("out")).unwrap();
        let rep = run(&a.path).unwrap();
        assert_eq!((rep.players, rep.files), (1, 3));
        let leftovers = std::fs::read_dir(std::env::temp_dir()).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().starts_with(&format!("palmanager-restoretest-{}", std::process::id()))).count();
        assert_eq!(leftovers, 0);
        // Level.sav vide
        let root2 = std::env::temp_dir().join(format!("pal-rt2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root2);
        make_world(&root2, b"", b"meta");
        let a2 = backup::create(&root2.join("SaveGames"), &root2.join("out")).unwrap();
        assert!(run(&a2.path).unwrap_err().to_string().contains("Level.sav"));
        std::fs::remove_dir_all(root).ok();
        std::fs::remove_dir_all(root2).ok();
    }
}
