//! Sauvegardes ZIP du dossier `SaveGames`, avec rotation.

use crate::{settings::AppSettings, Result};
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
    /// Sauvegarde protégée (« gardée ») : jamais supprimée par la rotation.
    pub protected: bool,
}

/// Une sauvegarde protégée porte `-garde` dans son nom.
pub fn is_protected(file_name: &str) -> bool { file_name.contains("-garde") }

static LAST_MIRROR_ERROR: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Dernière erreur de copie miroir (lue une seule fois par le superviseur pour alerter).
pub fn take_mirror_error() -> Option<String> { LAST_MIRROR_ERROR.lock().ok().and_then(|mut g| g.take()) }

/// Crée `palworld-YYYYmmdd-HHMMSS.zip` dans `dest`. À appeler après un `save` REST pour un état cohérent.
pub fn create(save_dir: &Path, dest: &Path) -> Result<BackupInfo> { create_labeled(save_dir, dest, None) }

/// Comme `create`, avec une étiquette dans le nom (`palworld-…-arret.zip`) pour savoir d'où vient la sauvegarde.
pub fn create_labeled(save_dir: &Path, dest: &Path, label: Option<&str>) -> Result<BackupInfo> {
    std::fs::create_dir_all(dest)?;
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    let name = match label {
        Some(l) => format!("palworld-{stamp}-{}.zip", l.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect::<String>()),
        None => format!("palworld-{stamp}.zip"),
    };
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
    let file_name: String = path.file_name().map(|n| n.to_string_lossy().into()).unwrap_or_default();
    Ok(BackupInfo {
        protected: is_protected(&file_name),
        file_name,
        path: path.to_path_buf(),
        size_bytes: md.len(),
        created: md.modified()?.into(),
    })
}

/// Sauvegarde complète : archive + rotation, puis copie vers le second emplacement s'il est configuré.
/// Une erreur de copie miroir n'invalide pas la sauvegarde principale (elle est seulement signalée sur stderr).
pub fn backup_all(s: &AppSettings, label: Option<&str>) -> Result<BackupInfo> {
    if !s.save_dir().exists() {
        return Err(crate::Error::Other(format!("dossier de sauvegarde introuvable : {}", s.save_dir().display())));
    }
    let info = create_labeled(&s.save_dir(), &s.backup.destination, label)?;
    // Une archive qu'on ne peut pas relire est pire que pas d'archive : on la supprime et on le dit.
    if let Err(e) = verify(&info.path) {
        let _ = std::fs::remove_file(&info.path);
        return Err(crate::Error::Other(format!("sauvegarde {} défectueuse, supprimée : {e}", info.file_name)));
    }
    rotate_policy(&s.backup.destination, &s.backup)?;
    if s.backup.player_snapshots {
        // Au mieux : un échec ici ne doit jamais faire échouer la sauvegarde du monde.
        let n = crate::playersaves::snapshot_all(&s.save_dir(), &s.backup.destination, s.backup.player_keep);
        if n > 0 { eprintln!("{n} sauvegarde(s) joueur créée(s)"); }
    }
    if let Some(m) = &s.backup.mirror_destination {
        if let Err(e) = mirror(&info.path, m, &s.backup) {
            eprintln!("copie miroir impossible : {e}");
            if let Ok(mut g) = LAST_MIRROR_ERROR.lock() { *g = Some(format!("copie vers {} impossible : {e}", m.display())); }
        }
    }
    Ok(info)
}

/// Copie une archive vers un second emplacement puis applique la même rotation.
pub fn mirror(archive: &Path, dest: &Path, cfg: &crate::settings::BackupSettings) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    if let Some(name) = archive.file_name() { std::fs::copy(archive, dest.join(name))?; }
    rotate_policy(dest, cfg)?;
    Ok(())
}

/// Quelles sauvegardes garder (même ordre que `backups`, la plus récente d'abord) : tout ce qui a moins de 24 h, puis la plus
/// récente de chaque jour jusqu'à 7 jours, puis la plus récente de chaque semaine jusqu'à 28 jours ; le reste est supprimé.
/// La toute dernière est toujours conservée.
pub fn plan_tiered(backups: &[BackupInfo], now: DateTime<Local>) -> Vec<bool> {
    use chrono::Datelike;
    let mut seen_days = std::collections::HashSet::new();
    let mut seen_weeks = std::collections::HashSet::new();
    backups.iter().enumerate().map(|(i, b)| {
        let age = now - b.created;
        if i == 0 || age < chrono::Duration::hours(24) { return true; }
        if age < chrono::Duration::days(7) { return seen_days.insert(b.created.date_naive()); }
        if age < chrono::Duration::days(28) { let w = b.created.iso_week(); return seen_weeks.insert((w.year(), w.week())); }
        false
    }).collect()
}

/// Rotation selon la politique choisie : par paliers, ou simplement les `retention` plus récentes.
pub fn rotate_policy(dest: &Path, cfg: &crate::settings::BackupSettings) -> Result<usize> {
    if !cfg.tiered { return rotate(dest, cfg.retention); }
    let all: Vec<BackupInfo> = list(dest)?.into_iter().filter(|b| !b.protected).collect(); // les sauvegardes protégées ne comptent pas
    let keep = plan_tiered(&all, Local::now());
    let mut n = 0;
    for (b, k) in all.iter().zip(keep) { if !k { std::fs::remove_file(&b.path)?; n += 1; } }
    Ok(n)
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

/// Protège (ou déprotège) une sauvegarde en renommant le fichier ; renvoie son nouveau chemin.
pub fn set_protected(path: &Path, on: bool) -> Result<PathBuf> {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if !name.ends_with(".zip") { return Err(crate::Error::Other("ce n'est pas une sauvegarde".into())); }
    let new = if on && !is_protected(&name) { name.trim_end_matches(".zip").to_string() + "-garde.zip" }
        else if !on && is_protected(&name) { name.replacen("-garde", "", 1) } else { return Ok(path.to_path_buf()) };
    let target = path.with_file_name(new);
    if target.exists() { return Err(crate::Error::Other("une sauvegarde de ce nom existe déjà".into())); }
    std::fs::rename(path, &target)?;
    Ok(target)
}

/// Ne garde que les `keep` plus récentes ; renvoie le nombre supprimé.
pub fn rotate(dest: &Path, keep: usize) -> Result<usize> {
    let old: Vec<_> = list(dest)?.into_iter().filter(|b| !b.protected).skip(keep.max(1)).collect();
    for b in &old { std::fs::remove_file(&b.path)?; }
    Ok(old.len())
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VerifyReport { pub files: usize, pub bytes: u64 }

/// Lit entièrement l'archive (ce qui contrôle les CRC) sans rien écrire. Refuse une archive vide, illisible
/// ou contenant un chemin dangereux : on ne remplace jamais un monde sain par une archive douteuse.
pub fn verify(archive: &Path) -> Result<VerifyReport> {
    let mut zip = zip::ZipArchive::new(File::open(archive)?)?;
    let (mut files, mut bytes) = (0usize, 0u64);
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        if f.enclosed_name().is_none() { return Err(crate::Error::Other(format!("chemin dangereux dans l'archive : {}", f.name()))); }
        if f.is_dir() { continue; }
        let n = std::io::copy(&mut f, &mut std::io::sink())
            .map_err(|e| crate::Error::Other(format!("archive corrompue ({}) : {e}", f.name())))?;
        files += 1;
        bytes += n;
    }
    if files == 0 { return Err(crate::Error::Other("archive vide".into())); }
    Ok(VerifyReport { files, bytes })
}

/// Restaure une archive dans `save_dir` (le serveur doit être arrêté). L'existant est déplacé en `SaveGames.bak`.
pub fn restore(archive: &Path, save_dir: &Path) -> Result<()> {
    verify(archive)?; // avant de toucher au monde existant
    let bak = save_dir.with_extension("bak");
    if bak.exists() { std::fs::remove_dir_all(&bak)?; }
    if save_dir.exists() { std::fs::rename(save_dir, &bak)?; }
    restore_into(archive, save_dir)
}

/// Extrait l'archive dans `dir` (créé au besoin) sans toucher à rien d'autre ; chemins dangereux ignorés (anti zip-slip).
pub fn restore_into(archive: &Path, dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let mut zip = zip::ZipArchive::new(File::open(archive)?)?;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        let Some(rel) = f.enclosed_name().map(|p| p.to_path_buf()) else { continue };
        if f.is_dir() { continue; }
        let out = dir.join(rel);
        if let Some(p) = out.parent() { std::fs::create_dir_all(p)?; }
        std::io::copy(&mut f, &mut File::create(out)?)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake(age_h: i64) -> BackupInfo {
        BackupInfo { file_name: format!("b{age_h}.zip"), path: format!("b{age_h}.zip").into(), size_bytes: 1, created: Local::now() - chrono::Duration::hours(age_h), protected: false }
    }

    #[test]
    fn protected_backups_survive_rotation_and_can_be_toggled() {
        let tmp = std::env::temp_dir().join(format!("pal-prot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        for (i, n) in ["palworld-20260101-000001.zip", "palworld-20260101-000002-garde-boss.zip", "palworld-20260101-000003.zip", "palworld-20260101-000004.zip"].iter().enumerate() {
            std::fs::write(tmp.join(n), b"x").unwrap();
            let t = std::time::SystemTime::now() - std::time::Duration::from_secs(1000 - i as u64 * 100);
            std::fs::File::options().write(true).open(tmp.join(n)).unwrap().set_modified(t).unwrap();
        }
        assert_eq!(rotate(&tmp, 1).unwrap(), 2); // garde 1 non protégée (la plus récente) ; la protégée reste
        let left: Vec<String> = list(&tmp).unwrap().into_iter().map(|b| b.file_name).collect();
        assert!(left.contains(&"palworld-20260101-000002-garde-boss.zip".to_string()) && left.len() == 2, "{left:?}");
        let p = set_protected(&tmp.join("palworld-20260101-000004.zip"), true).unwrap();
        assert!(p.to_string_lossy().ends_with("000004-garde.zip") && list(&tmp).unwrap().iter().filter(|b| b.protected).count() == 2);
        let back = set_protected(&p, false).unwrap();
        assert!(back.ends_with("palworld-20260101-000004.zip"));
        std::fs::remove_dir_all(tmp).ok();
    }

    #[test]
    fn tiered_plan_keeps_recent_then_daily_then_weekly() {
        // du plus récent au plus ancien
        let ages = [1, 2, 5, 20, 30, 34, 60, 80, 200, 210, 400, 700];
        let list: Vec<BackupInfo> = ages.iter().map(|h| fake(*h)).collect();
        let keep = plan_tiered(&list, Local::now());
        let kept: Vec<i64> = ages.iter().zip(&keep).filter(|(_, k)| **k).map(|(h, _)| *h).collect();
        assert!(kept.starts_with(&[1, 2, 5, 20]), "{kept:?}"); // < 24 h : tout
        assert!(!kept.contains(&700) && kept.contains(&400), "{kept:?}"); // 700 h ≈ 29 j : supprimé ; 400 h ≈ 17 j : gardé (palier hebdomadaire)
        assert!(kept.len() < ages.len());
        // une seule sauvegarde par jour entre 1 et 7 jours
        let days: Vec<_> = list.iter().zip(&keep).filter(|(b, k)| **k && (Local::now() - b.created) >= chrono::Duration::hours(24) && (Local::now() - b.created) < chrono::Duration::days(7)).map(|(b, _)| b.created.date_naive()).collect();
        assert_eq!(days.len(), days.iter().collect::<std::collections::HashSet<_>>().len());
        assert!(plan_tiered(&[fake(900)], Local::now())[0]); // la dernière est toujours gardée
    }

    #[test]
    fn verify_accepts_good_and_rejects_bad_archives() {
        let tmp = std::env::temp_dir().join(format!("pal-vf-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("SaveGames/0")).unwrap();
        std::fs::write(tmp.join("SaveGames/0/Level.sav"), vec![7u8; 5000]).unwrap();
        let info = create(&tmp.join("SaveGames"), &tmp.join("out")).unwrap();
        let rep = verify(&info.path).unwrap();
        assert_eq!((rep.files, rep.bytes), (1, 5000));
        // tronquée
        let mut data = std::fs::read(&info.path).unwrap();
        data.truncate(data.len() / 2);
        let bad = tmp.join("bad.zip");
        std::fs::write(&bad, &data).unwrap();
        assert!(verify(&bad).is_err());
        // un restore refusé ne touche pas au monde existant
        assert!(restore(&bad, &tmp.join("SaveGames")).is_err());
        assert!(tmp.join("SaveGames/0/Level.sav").exists());
        // vide
        let empty = tmp.join("empty.zip");
        zip::ZipWriter::new(File::create(&empty).unwrap()).finish().unwrap();
        assert!(verify(&empty).is_err());
        std::fs::remove_dir_all(tmp).ok();
    }

    #[test]
    fn labeled_backup_all() {
        let tmp = std::env::temp_dir().join(format!("pal-bk2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let s = AppSettings {
            server_dir: tmp.clone(),
            backup: crate::settings::BackupSettings { destination: tmp.join("out"), mirror_destination: Some(tmp.join("mir")), retention: 3, ..Default::default() },
            ..Default::default()
        };
        assert!(backup_all(&s, Some("arret")).is_err()); // pas de dossier SaveGames
        std::fs::create_dir_all(s.save_dir().join("0")).unwrap();
        std::fs::write(s.save_dir().join("0/Level.sav"), b"x").unwrap();
        let info = backup_all(&s, Some("arret ../x")).unwrap();
        assert!(info.file_name.ends_with("-arretx.zip"), "{}", info.file_name); // étiquette assainie
        assert_eq!(list(&tmp.join("out")).unwrap().len(), 1);
        assert_eq!(list(&tmp.join("mir")).unwrap().len(), 1);
        std::fs::remove_dir_all(tmp).ok();
    }

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
        mirror(&list(&dest).unwrap()[0].path, &second, &crate::settings::BackupSettings { retention: 5, ..Default::default() }).unwrap();
        assert_eq!(list(&second).unwrap().len(), 1);
        let arc = list(&dest).unwrap().remove(0).path;
        std::fs::remove_file(saves.join("Level.sav")).unwrap();
        restore(&arc, &tmp.join("SaveGames")).unwrap();
        assert_eq!(std::fs::read(saves.join("Level.sav")).unwrap(), b"data");
        std::fs::remove_dir_all(tmp).ok();
    }
}
