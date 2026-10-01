//! Sauvegardes ZIP du dossier `SaveGames`, avec rotation.

use crate::Result;
use chrono::{DateTime, Local};
use serde::Serialize;
use std::{fs::File, io::{Read, Write}, path::{Path, PathBuf}};
use walkdir::WalkDir;
use zip::{write::FileOptions, ZipWriter};

#[derive(Debug, Clone, Serialize)]
pub struct BackupInfo {
    pub file_name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub created: DateTime<Local>,
}

/// Crée `palworld-YYYYmmdd-HHMMSS.zip` dans `dest`. À appeler après un `save` REST pour un état cohérent.
pub fn create(save_dir: &Path, dest: &Path) -> Result<BackupInfo> {
    std::fs::create_dir_all(dest)?;
    let name = format!("palworld-{}.zip", Local::now().format("%Y%m%d-%H%M%S"));
    let path = dest.join(&name);
    let mut zip = ZipWriter::new(File::create(&path)?);
    let opts = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut buf = Vec::new();
    for e in WalkDir::new(save_dir).into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()) {
        let rel = e.path().strip_prefix(save_dir).unwrap_or(e.path()).to_string_lossy().replace('\\', "/");
        zip.start_file(rel, opts)?;
        buf.clear();
        File::open(e.path())?.read_to_end(&mut buf)?;
        zip.write_all(&buf)?;
    }
    zip.finish()?;
    info_for(&path)
}

fn info_for(path: &Path) -> Result<BackupInfo> {
    let md = std::fs::metadata(path)?;
    Ok(BackupInfo {
        file_name: path.file_name().map(|n| n.to_string_lossy().into()).unwrap_or_default(),
        path: path.to_path_buf(),
        size_bytes: md.len(),
        created: md.modified()?.into(),
    })
}

/// Copie une archive vers un second emplacement puis applique la même rotation.
pub fn mirror(archive: &Path, dest: &Path, keep: usize) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    if let Some(name) = archive.file_name() { std::fs::copy(archive, dest.join(name))?; }
    rotate(dest, keep)?;
    Ok(())
}

/// Sauvegardes existantes, la plus récente d'abord.
pub fn list(dest: &Path) -> Result<Vec<BackupInfo>> {
    let mut v = Vec::new();
    if dest.exists() {
        for e in std::fs::read_dir(dest)? {
            let p = e?.path();
            if p.extension().is_some_and(|x| x == "zip") { v.push(info_for(&p)?); }
        }
    }
    v.sort_by(|a, b| b.created.cmp(&a.created));
    Ok(v)
}

/// Ne garde que les `keep` plus récentes ; renvoie le nombre supprimé.
pub fn rotate(dest: &Path, keep: usize) -> Result<usize> {
    let old: Vec<_> = list(dest)?.into_iter().skip(keep.max(1)).collect();
    for b in &old { std::fs::remove_file(&b.path)?; }
    Ok(old.len())
}

/// Restaure une archive dans `save_dir` (le serveur doit être arrêté). L'existant est déplacé en `SaveGames.bak`.
pub fn restore(archive: &Path, save_dir: &Path) -> Result<()> {
    let bak = save_dir.with_extension("bak");
    if bak.exists() { std::fs::remove_dir_all(&bak)?; }
    if save_dir.exists() { std::fs::rename(save_dir, &bak)?; }
    std::fs::create_dir_all(save_dir)?;
    let mut zip = zip::ZipArchive::new(File::open(archive)?)?;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        let Some(rel) = f.enclosed_name().map(|p| p.to_path_buf()) else { continue }; // anti zip-slip
        let out = save_dir.join(rel);
        if let Some(p) = out.parent() { std::fs::create_dir_all(p)?; }
        std::io::copy(&mut f, &mut File::create(out)?)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_rotate_restore() {
        let tmp = std::env::temp_dir().join(format!("pal-bk-{}", std::process::id()));
        let saves = tmp.join("SaveGames/0/W");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::write(saves.join("Level.sav"), b"data").unwrap();
        let dest = tmp.join("out");
        create(&tmp.join("SaveGames"), &dest).unwrap();
        assert_eq!(list(&dest).unwrap().len(), 1);
        assert_eq!(rotate(&dest, 5).unwrap(), 0);
        let second = tmp.join("mirror");
        mirror(&list(&dest).unwrap()[0].path, &second, 5).unwrap();
        assert_eq!(list(&second).unwrap().len(), 1);
        let arc = list(&dest).unwrap().remove(0).path;
        std::fs::remove_file(saves.join("Level.sav")).unwrap();
        restore(&arc, &tmp.join("SaveGames")).unwrap();
        assert_eq!(std::fs::read(saves.join("Level.sav")).unwrap(), b"data");
        std::fs::remove_dir_all(tmp).ok();
    }
}
