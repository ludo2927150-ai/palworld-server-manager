//! Sauvegardes individuelles des fichiers joueur (`SaveGames/<slot>/<monde>/Players/<id>.sav` et `<id>_dps.sav`).
//!
//! Limite importante : ces fichiers contiennent la fiche du joueur (niveau, statistiques, technologies, position…), mais
//! ses objets, ses Pals et ses bases sont stockés dans `Level.sav`. Restaurer un fichier joueur ne ramène donc que sa
//! fiche, dans le MÊME monde ; une restauration complète d'un joueur reste celle de la sauvegarde du monde.
//! Aucun fichier de sauvegarde du jeu n'est modifié par ce module, hormis la restauration explicite d'un fichier joueur.

use crate::{Error, Result};
use chrono::{DateTime, Local, TimeZone};
use serde::Serialize;
use std::{collections::BTreeSet, fs::File, io::Write, path::{Path, PathBuf}};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PlayerSnapshot {
    pub file_name: String,
    pub path: PathBuf,
    pub taken: DateTime<Local>,
    pub size_bytes: u64,
}

/// Identifiant joueur du jeu : 32 caractères hexadécimaux.
pub fn valid_id(id: &str) -> bool { id.len() == 32 && id.chars().all(|c| c.is_ascii_hexdigit()) }

/// Nom de fichier joueur autorisé : `<id>.sav` ou `<id>_dps.sav`.
fn player_file_id(name: &str) -> Option<String> {
    let stem = name.strip_suffix(".sav")?;
    let id = stem.strip_suffix("_dps").unwrap_or(stem);
    valid_id(id).then(|| id.to_ascii_uppercase())
}

/// Dossiers `Players` de tous les mondes : `<save_dir>/<slot>/<monde>/Players`.
pub fn players_dirs(save_dir: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    let Ok(slots) = std::fs::read_dir(save_dir) else { return v };
    for slot in slots.flatten().filter(|e| e.path().is_dir()) {
        let Ok(worlds) = std::fs::read_dir(slot.path()) else { continue };
        for w in worlds.flatten() { let p = w.path().join("Players"); if p.is_dir() { v.push(p); } }
    }
    v
}

/// Identifiants des joueurs ayant un fichier dans le monde.
pub fn known_ids(save_dir: &Path) -> BTreeSet<String> {
    players_dirs(save_dir).iter().filter_map(|d| std::fs::read_dir(d).ok()).flat_map(|rd| rd.flatten())
        .filter_map(|e| player_file_id(&e.file_name().to_string_lossy())).collect()
}

/// Fichiers du joueur (`.sav` et `_dps.sav`) avec leur chemin relatif à `save_dir`.
fn files_for(save_dir: &Path, id: &str) -> Vec<(PathBuf, String)> {
    let want = id.to_ascii_uppercase();
    let mut out = Vec::new();
    for dir in players_dirs(save_dir) {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if player_file_id(&name).as_deref() == Some(want.as_str()) && e.path().is_file() {
                let rel = e.path().strip_prefix(save_dir).map(|r| r.to_string_lossy().replace('\\', "/")).unwrap_or(name);
                out.push((e.path(), rel));
            }
        }
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out
}

fn fnv(bytes: &[u8], mut h: u64) -> u64 { for b in bytes { h ^= *b as u64; h = h.wrapping_mul(0x0000_0100_0000_01b3); } h }

/// Dossier des sauvegardes d'un joueur : `<dest>/joueurs/<ID>/`.
pub fn player_dir(dest: &Path, id: &str) -> PathBuf { dest.join("joueurs").join(id.to_ascii_uppercase()) }

pub fn list(dest: &Path, id: &str) -> Vec<PlayerSnapshot> {
    if !valid_id(id) { return vec![]; }
    let Ok(rd) = std::fs::read_dir(player_dir(dest, id)) else { return vec![] };
    let mut v: Vec<PlayerSnapshot> = rd.flatten().filter_map(|e| {
        let p = e.path();
        let name = p.file_name()?.to_string_lossy().to_string();
        let stamp = name.strip_suffix(".zip")?.split('-').take(2).collect::<Vec<_>>().join("-");
        let naive = chrono::NaiveDateTime::parse_from_str(&stamp, "%Y%m%d-%H%M%S%3f").ok()?;
        Some(PlayerSnapshot { taken: Local.from_local_datetime(&naive).single()?, size_bytes: e.metadata().ok()?.len(), file_name: name, path: p })
    }).collect();
    v.sort_by(|a, b| b.taken.cmp(&a.taken));
    v
}

/// Sauvegarde les fichiers d'un joueur. `None` si rien n'a changé depuis la dernière sauvegarde (ou s'il n'a pas de fichier).
pub fn snapshot_player(save_dir: &Path, dest: &Path, id: &str, keep: usize) -> Result<Option<PlayerSnapshot>> {
    if !valid_id(id) { return Err(Error::Other("identifiant joueur invalide".into())); }
    let files = files_for(save_dir, id);
    if files.is_empty() { return Ok(None); }
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut blobs = Vec::new();
    for (path, rel) in &files {
        let data = std::fs::read(path)?;
        hash = fnv(rel.as_bytes(), hash);
        hash = fnv(&data, hash);
        blobs.push((rel.clone(), data));
    }
    let tag = format!("{:08x}", hash as u32);
    let dir = player_dir(dest, id);
    if list(dest, id).first().is_some_and(|l| l.file_name.contains(&format!("-{tag}.zip"))) { return Ok(None); }
    std::fs::create_dir_all(&dir)?;
    let name = format!("{}-{tag}.zip", Local::now().format("%Y%m%d-%H%M%S%3f"));
    let path = dir.join(&name);
    let mut zip = zip::ZipWriter::new(File::create(&path)?);
    for (rel, data) in &blobs {
        zip.start_file(rel.as_str(), zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated))?;
        zip.write_all(data)?;
    }
    zip.finish()?;
    for old in list(dest, id).into_iter().skip(keep.max(1)) { let _ = std::fs::remove_file(old.path); }
    Ok(list(dest, id).into_iter().find(|s| s.file_name == name))
}

/// Sauvegarde tous les joueurs qui ont un fichier ; renvoie le nombre de nouvelles sauvegardes.
pub fn snapshot_all(save_dir: &Path, dest: &Path, keep: usize) -> usize {
    known_ids(save_dir).iter().filter(|id| matches!(snapshot_player(save_dir, dest, id, keep), Ok(Some(_)))).count()
}

/// Entrée d'archive acceptée : `<slot>/<monde>/Players/<fichier joueur>`, chaque élément validé (aucun `..`, aucun chemin absolu).
fn safe_rel(entry: &str) -> Option<(PathBuf, String)> {
    let parts: Vec<&str> = entry.split('/').collect();
    let [slot, world, players, file] = parts.as_slice() else { return None };
    let plain = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    (plain(slot) && plain(world) && *players == "Players" && player_file_id(file).is_some()).then(|| (PathBuf::from(slot).join(world).join(players).join(file), file.to_string()))
}

/// Restaure les fichiers d'une sauvegarde joueur dans le monde (le serveur doit être arrêté). Le dossier du monde doit exister :
/// on ne crée jamais un monde. Chaque fichier remplacé est conservé en `<fichier>.avant-restauration`. Renvoie les fichiers écrits.
pub fn restore_player(archive: &Path, save_dir: &Path) -> Result<Vec<String>> {
    let mut zip = zip::ZipArchive::new(File::open(archive)?)?;
    let mut plan = Vec::new();
    for i in 0..zip.len() {
        let f = zip.by_index(i)?;
        if f.is_dir() { continue; }
        let (rel, file) = safe_rel(f.name()).ok_or_else(|| Error::Other(format!("entrée d'archive refusée : {}", f.name())))?;
        let target = save_dir.join(&rel);
        if !target.parent().is_some_and(|p| p.is_dir()) {
            return Err(Error::Other(format!("le monde de cette sauvegarde est introuvable sur ce serveur ({}) : un fichier joueur ne se restaure que dans le même monde", rel.parent().and_then(|p| p.parent()).map(|p| p.display().to_string()).unwrap_or_default())));
        }
        plan.push((i, target, file));
    }
    if plan.is_empty() { return Err(Error::Other("archive sans fichier joueur".into())); }
    let mut written = Vec::new();
    for (i, target, file) in plan {
        let mut f = zip.by_index(i)?;
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut data)?;
        if target.exists() { std::fs::copy(&target, target.with_extension("sav.avant-restauration"))?; }
        let tmp = target.with_extension("sav.tmp");
        std::fs::write(&tmp, &data)?;
        std::fs::rename(&tmp, &target)?;
        written.push(file);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "F8A7388F000000000000000000000000";

    fn world(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("pal-ps-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let players = root.join("SaveGames/0/WORLDGUID123/Players");
        std::fs::create_dir_all(&players).unwrap();
        (root.clone(), root.join("SaveGames"), players)
    }

    #[test]
    fn snapshot_dedup_rotation_and_restore() {
        let (root, save, players) = world("a");
        std::fs::write(players.join(format!("{ID}.sav")), b"v1").unwrap();
        std::fs::write(players.join(format!("{ID}_dps.sav")), b"dps").unwrap();
        std::fs::write(players.join("0123456789ABCDEF0123456789ABCDEF.sav"), b"other").unwrap();
        std::fs::write(players.join("notes.txt"), b"x").unwrap();
        let dest = root.join("backups");
        assert_eq!(known_ids(&save).len(), 2);
        let s1 = snapshot_player(&save, &dest, ID, 2).unwrap().unwrap();
        assert!(snapshot_player(&save, &dest, ID, 2).unwrap().is_none(), "inchangé : pas de doublon");
        std::thread::sleep(std::time::Duration::from_millis(5)); // horodatage à la milliseconde
        std::fs::write(players.join(format!("{ID}.sav")), b"v2").unwrap();
        let s2 = snapshot_player(&save, &dest, ID, 2).unwrap().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        std::fs::write(players.join(format!("{ID}.sav")), b"v3").unwrap();
        snapshot_player(&save, &dest, ID, 2).unwrap().unwrap();
        let l = list(&dest, ID);
        assert_eq!(l.len(), 2, "rotation : 2 gardées");
        assert!(!s1.path.exists() && s2.path.exists());
        // le fichier est perdu : on le restaure depuis la plus récente
        std::fs::remove_file(players.join(format!("{ID}.sav"))).unwrap();
        let written = restore_player(&l[0].path, &save).unwrap();
        assert_eq!(written.len(), 2);
        assert_eq!(std::fs::read(players.join(format!("{ID}.sav"))).unwrap(), b"v3");
        // un fichier existant est conservé avant remplacement
        restore_player(&l[1].path, &save).unwrap();
        assert_eq!(std::fs::read(players.join(format!("{ID}.sav"))).unwrap(), b"v2");
        assert_eq!(std::fs::read(players.join(format!("{ID}.sav.avant-restauration"))).unwrap(), b"v3");
        assert_eq!(snapshot_all(&save, &dest, 2), 2, "le premier a changé (restauré en v2), l'autre est nouveau");
        assert_eq!(snapshot_all(&save, &dest, 2), 0, "plus rien à sauvegarder");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn restore_refuses_unknown_world_and_hostile_entries() {
        let (root, save, _players) = world("b");
        let zip_with = |entry: &str| {
            let p = root.join(format!("{}.zip", entry.replace(['/', '.'], "_")));
            let mut z = zip::ZipWriter::new(File::create(&p).unwrap());
            z.start_file(entry, zip::write::FileOptions::default()).unwrap();
            z.write_all(b"x").unwrap();
            z.finish().unwrap();
            p
        };
        // monde absent de ce serveur
        let e = restore_player(&zip_with(&format!("0/AUTREMONDE/Players/{ID}.sav")), &save).unwrap_err().to_string();
        assert!(e.contains("même monde"), "{e}");
        // chemins hostiles / fichiers non joueur
        for bad in [format!("../0/WORLDGUID123/Players/{ID}.sav"), "0/WORLDGUID123/Level.sav".to_string(), "0/WORLDGUID123/Players/evil.sav".to_string(), format!("/abs/Players/{ID}.sav")] {
            assert!(restore_player(&zip_with(&bad), &save).is_err(), "{bad}");
        }
        assert!(snapshot_player(&save, &root, "../x", 3).is_err());
        assert!(list(&root, "pas-un-id").is_empty());
        std::fs::remove_dir_all(root).ok();
    }
}
