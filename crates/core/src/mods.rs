//! Mods Palworld 1.0 (chargeur officiel de Pocketpair). Spécification utilisée :
//! - `Mods\PalModSettings.ini` : `bGlobalEnableMod`, `WorkshopRootDir`, une ligne `ActiveModList=<PackageName>` par mod activé ;
//! - chaque mod du Workshop est un dossier `<WorkshopRootDir>\<id>\` contenant un `Info.json` (PackageName, Version, InstallRule…) ;
//! - un mod ne tourne sur un serveur dédié que s'il déclare une règle d'installation avec `"IsServer": true`.
//!
//! Les mods sont lus au démarrage du serveur : tout changement demande un redémarrage.

use crate::{Error, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Identifiant Steam de Palworld, qui désigne le dossier `workshop\content\<id>`.
pub const APP_ID: &str = "1623730";

// ───────────────────────── PalModSettings.ini ─────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModSettings {
    pub global_enable: bool,
    pub workshop_root: Option<String>,
    pub active: Vec<String>,
    /// Lignes inconnues, conservées telles quelles à l'écriture (rien n'est perdu).
    #[serde(skip)]
    other: Vec<String>,
}

impl Default for ModSettings {
    fn default() -> Self { Self { global_enable: true, workshop_root: None, active: Vec::new(), other: Vec::new() } }
}

pub fn settings_path(server_dir: &Path) -> PathBuf { server_dir.join("Mods").join("PalModSettings.ini") }

pub fn parse_settings(text: &str) -> ModSettings {
    let mut s = ModSettings::default();
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() || l.eq_ignore_ascii_case("[PalModSettings]") { continue; }
        match l.split_once('=') {
            Some((k, v)) if k.trim().eq_ignore_ascii_case("bGlobalEnableMod") => s.global_enable = v.trim().eq_ignore_ascii_case("true"),
            Some((k, v)) if k.trim().eq_ignore_ascii_case("WorkshopRootDir") => s.workshop_root = Some(v.trim().to_string()).filter(|x| !x.is_empty()),
            Some((k, v)) if k.trim().eq_ignore_ascii_case("ActiveModList") => {
                let v = v.trim().to_string();
                if !v.is_empty() && !s.active.contains(&v) { s.active.push(v); }
            }
            _ => s.other.push(l.to_string()),
        }
    }
    s
}

pub fn render_settings(s: &ModSettings) -> String {
    let mut out = vec![
        "[PalModSettings]".to_string(),
        format!("bGlobalEnableMod={}", if s.global_enable { "True" } else { "False" }),
        format!("WorkshopRootDir={}", s.workshop_root.clone().unwrap_or_default()),
    ];
    out.extend(s.active.iter().map(|p| format!("ActiveModList={p}")));
    out.extend(s.other.iter().cloned());
    out.join("\r\n") + "\r\n"
}

pub fn load_settings(server_dir: &Path) -> ModSettings {
    std::fs::read_to_string(settings_path(server_dir)).map(|t| parse_settings(&t)).unwrap_or_default()
}

/// Écrit le fichier (copie `.bak` de l'ancien contenu).
pub fn save_settings(server_dir: &Path, s: &ModSettings) -> Result<()> {
    let p = settings_path(server_dir);
    if let Some(dir) = p.parent() { std::fs::create_dir_all(dir)?; }
    if p.exists() { std::fs::copy(&p, p.with_extension("ini.bak"))?; }
    std::fs::write(p, render_settings(s))?;
    Ok(())
}

// ───────────────────────── Info.json / dossiers du Workshop ─────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ModInfo {
    pub workshop_id: String,
    pub package_name: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    /// `true` si le mod déclare une règle `IsServer: true` (donc utilisable sur un serveur dédié).
    pub server_compatible: bool,
    pub active: bool,
    /// `true` si le dossier est sous le dossier du serveur (téléchargé par l'application) : on peut le supprimer.
    pub removable: bool,
    pub path: PathBuf,
}

fn get_ci<'a>(v: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    v.as_object()?.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, x)| x)
}

fn str_ci(v: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|k| get_ci(v, k)?.as_str().map(str::to_string)).filter(|s| !s.trim().is_empty())
}

/// Lit un `Info.json` (clés insensibles à la casse). `None` si le fichier est illisible ou sans `PackageName`.
pub fn parse_info(workshop_id: &str, json: &str, path: &Path) -> Option<ModInfo> {
    let v: serde_json::Value = serde_json::from_str(json.trim_start_matches('\u{feff}')).ok()?;
    let package_name = str_ci(&v, &["PackageName"])?;
    let server_compatible = get_ci(&v, "InstallRule").and_then(|r| r.as_array()).is_some_and(|rules| {
        rules.iter().any(|r| get_ci(r, "IsServer").and_then(|b| b.as_bool()).unwrap_or(false))
    });
    Some(ModInfo {
        workshop_id: workshop_id.to_string(),
        package_name,
        name: str_ci(&v, &["ModName", "Name", "Title"]),
        version: str_ci(&v, &["Version"]),
        author: str_ci(&v, &["Author"]),
        server_compatible,
        active: false,
        removable: false,
        path: path.to_path_buf(),
    })
}

/// Parcourt `<root>/<id>/Info.json` (jusqu'à 2 niveaux de profondeur) et marque les mods actifs.
pub fn scan(root: &Path, server_dir: &Path, active: &[String]) -> Vec<ModInfo> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else { return out };
    let server_dir = server_dir.canonicalize().unwrap_or_else(|_| server_dir.to_path_buf());
    for e in entries.flatten().filter(|e| e.path().is_dir()) {
        let id = e.file_name().to_string_lossy().to_string();
        let mut candidates = vec![e.path().join("Info.json")];
        if let Ok(sub) = std::fs::read_dir(e.path()) {
            candidates.extend(sub.flatten().filter(|s| s.path().is_dir()).map(|s| s.path().join("Info.json")));
        }
        if let Some(mut m) = candidates.iter().find_map(|c| std::fs::read_to_string(c).ok().and_then(|t| parse_info(&id, &t, &e.path()))) {
            m.active = active.iter().any(|a| a == &m.package_name);
            m.removable = id.chars().all(|c| c.is_ascii_digit()) && e.path().canonicalize().is_ok_and(|p| p.starts_with(&server_dir));
            out.push(m);
        }
    }
    out.sort_by(|a, b| a.name.clone().unwrap_or_else(|| a.package_name.clone()).to_lowercase().cmp(&b.name.clone().unwrap_or_else(|| b.package_name.clone()).to_lowercase()));
    out
}

/// Active ou désactive un mod (par `PackageName`). Un doublon n'est jamais ajouté.
pub fn set_active(s: &mut ModSettings, package_name: &str, on: bool) {
    s.active.retain(|p| p != package_name);
    if on { s.active.push(package_name.to_string()); }
}

/// Dossiers du Workshop plausibles pour ce serveur : celui rempli par SteamCMD dans le dossier du serveur,
/// puis ceux du client Steam. Seuls ceux qui existent sont renvoyés.
pub fn candidate_roots(server_dir: &Path) -> Vec<PathBuf> {
    let rel = Path::new("steamapps").join("workshop").join("content").join(APP_ID);
    let mut v = vec![server_dir.join(&rel)];
    for base in [r"C:\Program Files (x86)\Steam", r"C:\Program Files\Steam", r"D:\Steam", r"D:\SteamLibrary"] { v.push(Path::new(base).join(&rel)); }
    v.into_iter().filter(|p| p.is_dir()).collect()
}

/// Dossier à utiliser par défaut pour un téléchargement via SteamCMD (celui du serveur).
pub fn download_root(server_dir: &Path) -> PathBuf {
    server_dir.join("steamapps").join("workshop").join("content").join(APP_ID)
}

/// Accepte un identifiant (`3123456789`) ou une adresse Workshop (`…/sharedfiles/filedetails/?id=3123456789`).
pub fn parse_workshop_id(input: &str) -> Option<String> {
    let t = input.trim();
    if !t.is_empty() && t.len() <= 20 && t.chars().all(|c| c.is_ascii_digit()) { return Some(t.to_string()); }
    let q = t.split_once('?')?.1;
    let id = q.split('&').find_map(|kv| kv.strip_prefix("id="))?;
    (!id.is_empty() && id.len() <= 20 && id.chars().all(|c| c.is_ascii_digit())).then(|| id.to_string())
}

/// Supprime le dossier d'un mod téléchargé par l'application (sous `<serveur>\steamapps\workshop\content\1623730\<id>`).
pub fn remove_downloaded(server_dir: &Path, workshop_id: &str) -> Result<()> {
    if workshop_id.is_empty() || !workshop_id.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::Other("identifiant de mod invalide".into()));
    }
    let root = download_root(server_dir);
    let target = root.join(workshop_id);
    if !target.is_dir() { return Err(Error::Other("mod introuvable dans le dossier du serveur (les mods du client Steam se retirent depuis Steam)".into())); }
    if !target.canonicalize()?.starts_with(root.canonicalize()?) { return Err(Error::Other("chemin refusé".into())); }
    std::fs::remove_dir_all(target)?;
    Ok(())
}

/// Empreinte stable (FNV-1a) du contenu d'un dossier : chemins relatifs, tailles et dates. Sert à savoir si SteamCMD a mis un mod à jour.
pub fn dir_signature(dir: &Path) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |bytes: &[u8]| for b in bytes { h ^= *b as u64; h = h.wrapping_mul(0x0000_0100_0000_01b3); };
    let mut entries: Vec<_> = walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()).collect();
    entries.sort_by(|a, b| a.path().cmp(b.path()));
    for e in entries {
        let rel = e.path().strip_prefix(dir).unwrap_or(e.path()).to_string_lossy().replace('\\', "/");
        let (len, mtime) = e.metadata().map(|m| (m.len(), m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs()))).unwrap_or((0, 0));
        feed(rel.as_bytes()); feed(&len.to_le_bytes()); feed(&mtime.to_le_bytes());
    }
    h
}

/// Active exactement les mods du pack qui existent et fonctionnent sur un serveur ; renvoie les noms de paquets introuvables ou incompatibles.
pub fn apply_pack(s: &mut ModSettings, pack: &[String], available: &[ModInfo]) -> Vec<String> {
    let mut missing = Vec::new();
    s.active.clear();
    for name in pack {
        match available.iter().find(|m| &m.package_name == name) {
            Some(m) if m.server_compatible => set_active(s, name, true),
            _ => missing.push(name.clone()),
        }
    }
    missing
}

/// Mods activés dans PalModSettings.ini mais absents du dossier Workshop : le serveur risque de ne pas démarrer.
pub fn missing_active(s: &ModSettings, available: &[ModInfo]) -> Vec<String> {
    s.active.iter().filter(|a| !available.iter().any(|m| &m.package_name == *a)).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const INI: &str = "[PalModSettings]\r\nbGlobalEnableMod=True\r\nWorkshopRootDir=C:\\x\\1623730\r\nActiveModList=A\r\nActiveModList=B\r\nActiveModList=A\r\nAutreCle=1\r\n";

    #[test]
    fn settings_roundtrip_keeps_unknown_lines_and_dedups() {
        let s = parse_settings(INI);
        assert!(s.global_enable && s.workshop_root.as_deref() == Some("C:\\x\\1623730"));
        assert_eq!(s.active, ["A", "B"]); // doublon retiré
        let out = render_settings(&s);
        assert!(out.contains("AutreCle=1") && out.contains("ActiveModList=B") && out.starts_with("[PalModSettings]"));
        assert_eq!(parse_settings(&out), s);
    }

    #[test]
    fn set_active_toggles_without_duplicates() {
        let mut s = ModSettings::default();
        set_active(&mut s, "A", true); set_active(&mut s, "A", true); set_active(&mut s, "B", true);
        assert_eq!(s.active, ["A", "B"]);
        set_active(&mut s, "A", false);
        assert_eq!(s.active, ["B"]);
        assert!(!parse_settings("bGlobalEnableMod=False").global_enable);
        assert!(parse_settings("").global_enable); // défaut : mods activés
    }

    #[test]
    fn info_json_is_parsed_case_insensitively_and_detects_server_support() {
        let ok = r#"{"packagename":"SuperMod","Version":"1.2","Author":"Moi","InstallRule":[{"Type":"Paks","Targets":["./a.pak"]},{"Type":"Paks","Targets":["./b.pak"],"IsServer":true}]}"#;
        let m = parse_info("123", ok, Path::new("p")).unwrap();
        assert_eq!((m.package_name.as_str(), m.version.as_deref(), m.server_compatible), ("SuperMod", Some("1.2"), true));
        let client_only = r#"{"PackageName":"Skin","InstallRule":[{"Type":"Paks","Targets":["./a.pak"]}]}"#;
        assert!(!parse_info("1", client_only, Path::new("p")).unwrap().server_compatible);
        assert!(parse_info("1", "pas du json", Path::new("p")).is_none());
        assert!(parse_info("1", r#"{"Version":"1"}"#, Path::new("p")).is_none()); // sans PackageName
        assert!(parse_info("1", &format!("\u{feff}{ok}"), Path::new("p")).is_some()); // BOM
    }

    #[test]
    fn workshop_ids_from_urls() {
        assert_eq!(parse_workshop_id(" 3123456789 ").as_deref(), Some("3123456789"));
        assert_eq!(parse_workshop_id("https://steamcommunity.com/sharedfiles/filedetails/?id=3123456789&searchtext=").as_deref(), Some("3123456789"));
        assert_eq!(parse_workshop_id("https://steamcommunity.com/sharedfiles/filedetails/?searchtext=x&id=42").as_deref(), Some("42"));
        for bad in ["", "abc", "12 34", "https://x.y/?id=../..", "https://x.y/?id=", "-1"] { assert!(parse_workshop_id(bad).is_none(), "{bad}"); }
    }

    #[test]
    fn scan_marks_active_and_removable_and_remove_is_confined() {
        let server = std::env::temp_dir().join(format!("pal-mods-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&server);
        let root = download_root(&server);
        std::fs::create_dir_all(root.join("111")).unwrap();
        std::fs::write(root.join("111/Info.json"), r#"{"PackageName":"Alpha","ModName":"Alpha mod","InstallRule":[{"Type":"Lua","IsServer":true}]}"#).unwrap();
        std::fs::create_dir_all(root.join("222/sous")).unwrap(); // Info.json un niveau plus bas
        std::fs::write(root.join("222/sous/Info.json"), r#"{"PackageName":"Beta"}"#).unwrap();
        std::fs::create_dir_all(root.join("333")).unwrap();      // dossier sans Info.json : ignoré
        let mods = scan(&root, &server, &["Beta".to_string()]);
        assert_eq!(mods.len(), 2);
        let (a, b) = (&mods[0], &mods[1]);
        assert_eq!((a.package_name.as_str(), a.active, a.removable, a.server_compatible), ("Alpha", false, true, true));
        assert_eq!((b.package_name.as_str(), b.active, b.server_compatible), ("Beta", true, false));
        // Suppression : ids invalides refusés, dossier ciblé supprimé, le reste intact.
        for bad in ["", "..", "1/../2", "abc"] { assert!(remove_downloaded(&server, bad).is_err(), "{bad}"); }
        remove_downloaded(&server, "111").unwrap();
        assert!(!root.join("111").exists() && root.join("222").exists());
        // Écriture des réglages avec copie .bak.
        let mut s = ModSettings::default();
        set_active(&mut s, "Beta", true);
        save_settings(&server, &s).unwrap();
        save_settings(&server, &s).unwrap();
        assert!(settings_path(&server).with_extension("ini.bak").exists());
        assert_eq!(load_settings(&server).active, ["Beta"]);
        std::fs::remove_dir_all(server).ok();
    }

    fn info(pkg: &str, ok: bool) -> ModInfo {
        ModInfo { workshop_id: "1".into(), package_name: pkg.into(), name: None, version: None, author: None, server_compatible: ok, active: false, removable: true, path: PathBuf::new() }
    }

    #[test]
    fn packs_activate_only_existing_compatible_mods() {
        let avail = [info("A", true), info("B", false), info("C", true)];
        let mut s = ModSettings { active: vec!["Z".into()], ..ModSettings::default() };
        let missing = apply_pack(&mut s, &["A".into(), "B".into(), "D".into(), "C".into()], &avail);
        assert_eq!(s.active, ["A", "C"]);
        assert_eq!(missing, ["B", "D"]);
        assert_eq!(missing_active(&ModSettings { active: vec!["A".into(), "Q".into()], ..ModSettings::default() }, &avail), ["Q"]);
    }

    #[test]
    fn signature_changes_with_content_and_is_stable_otherwise() {
        let d = std::env::temp_dir().join(format!("pal-sig-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("sub")).unwrap();
        std::fs::write(d.join("Info.json"), "{}").unwrap();
        let a = dir_signature(&d);
        assert_eq!(a, dir_signature(&d));
        std::fs::write(d.join("sub/new.pak"), "x").unwrap();
        assert_ne!(a, dir_signature(&d));
        assert_ne!(dir_signature(&d), dir_signature(&d.join("nope")));
        std::fs::remove_dir_all(&d).ok();
    }
}
