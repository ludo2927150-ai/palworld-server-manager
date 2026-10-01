//! Profils de configuration du monde (« Normal », « Hardcore »…) : des jeux de réglages enregistrés que l'on
//! applique en un clic. L'identité du serveur (mots de passe, ports, API REST) n'est jamais écrasée.

use crate::{ini::{self, Options}, Error, Result};
use serde::Serialize;
use std::path::PathBuf;

/// Clés conservées telles quelles à l'application d'un profil.
const PROTECTED: [&str; 9] = ["AdminPassword", "ServerPassword", "RESTAPIEnabled", "RESTAPIPort", "RCONEnabled", "RCONPort", "PublicPort", "PublicIP", "Region"];

#[derive(Debug, Clone, Serialize)]
pub struct ProfileInfo { pub name: String, pub saved_at: i64, pub options: usize }

pub struct ProfileStore { dir: PathBuf }

fn clean(name: &str) -> Result<String> {
    let n: String = name.trim().chars().filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_')).take(40).collect();
    let n = n.trim().to_string();
    if n.is_empty() { Err(Error::Other("nom de profil vide ou invalide".into())) } else { Ok(n) }
}

impl ProfileStore {
    pub fn new(dir: PathBuf) -> Self { Self { dir } }
    fn path(&self, name: &str) -> Result<PathBuf> { Ok(self.dir.join(format!("{}.ini", clean(name)?))) }

    pub fn list(&self) -> Vec<ProfileInfo> {
        let Ok(rd) = std::fs::read_dir(&self.dir) else { return vec![] };
        let mut v: Vec<ProfileInfo> = rd.filter_map(|e| e.ok()).filter_map(|e| {
            let p = e.path();
            if p.extension()? != "ini" { return None; }
            let name = p.file_stem()?.to_string_lossy().to_string();
            let saved_at = e.metadata().ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
            let options = ini::parse(&std::fs::read_to_string(&p).ok()?).map(|o| o.len()).unwrap_or(0);
            Some(ProfileInfo { name, saved_at, options })
        }).collect();
        v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        v
    }

    pub fn save(&self, name: &str, options: &Options) -> Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        std::fs::write(self.path(name)?, ini::serialize(options))?;
        Ok(())
    }

    pub fn load(&self, name: &str) -> Result<Options> { ini::parse(&std::fs::read_to_string(self.path(name)?)?) }

    pub fn delete(&self, name: &str) -> Result<()> { std::fs::remove_file(self.path(name)?)?; Ok(()) }
}

/// Applique un profil sur les options courantes sans toucher aux clés protégées. Renvoie le nombre de valeurs modifiées.
pub fn apply(current: &mut Options, profile: &Options) -> usize {
    let mut changed = 0;
    for o in profile.iter().filter(|o| !PROTECTED.contains(&o.key.as_str())) {
        if ini::get(current, &o.key) != Some(o.value.as_str()) { changed += 1; }
        ini::set(current, &o.key, &o.value, o.quoted);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(s: &str) -> Options { ini::parse(&format!("[/Script/Pal.PalGameWorldSettings]\nOptionSettings=({s})\n")).unwrap() }

    #[test]
    fn save_list_load_delete_and_name_sanitizing() {
        let store = ProfileStore::new(std::env::temp_dir().join(format!("pal-prof-{}", std::process::id())));
        store.save("Hardcore ../x", &opts("ExpRate=0.5,DeathPenalty=All")).unwrap();
        let l = store.list();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].name, "Hardcore x");
        assert_eq!(ini::get(&store.load("Hardcore x").unwrap(), "ExpRate"), Some("0.5"));
        assert!(store.save("  ../ ", &opts("A=1")).is_err() || store.list().len() == 2); // le nom n'échappe jamais du dossier
        store.delete("Hardcore x").unwrap();
        assert!(store.load("Hardcore x").is_err());
        std::fs::remove_dir_all(&store.dir).ok();
    }

    #[test]
    fn apply_never_touches_protected_keys() {
        let mut cur = opts("ExpRate=1.0,AdminPassword=\"secret\",RESTAPIPort=8212,ServerName=\"A\"");
        let prof = opts("ExpRate=3.0,AdminPassword=\"hack\",RESTAPIPort=1,ServerName=\"B\",PalEggDefaultHatchingTime=0");
        assert_eq!(apply(&mut cur, &prof), 3); // ExpRate, ServerName, nouvelle clé
        assert_eq!(ini::get(&cur, "AdminPassword"), Some("secret"));
        assert_eq!(ini::get(&cur, "RESTAPIPort"), Some("8212"));
        assert_eq!(ini::get(&cur, "ExpRate"), Some("3.0"));
        assert_eq!(ini::get(&cur, "PalEggDefaultHatchingTime"), Some("0"));
    }
}
