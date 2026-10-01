//! Diagnostic de premier lancement : vérifie l'installation et corrige l'API REST en un clic.

use crate::{ini, rest::RestClient, settings::AppSettings, Error, Result};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct Check {
    pub id: &'static str,
    pub label: &'static str,
    pub ok: bool,
    pub detail: String,
}

fn chk(id: &'static str, label: &'static str, ok: bool, detail: impl Into<String>) -> Check {
    Check { id, label, ok, detail: detail.into() }
}

/// Vérifications de fichiers (synchrones).
pub fn diagnose_files(s: &AppSettings) -> Vec<Check> {
    let exe = s.exe_path();
    let ini_path = s.world_settings_path();
    let steamcmd = s.steamcmd_exe();
    let mut v = vec![
        chk("exe", "PalServer.exe trouvé", exe.exists(), exe.display().to_string()),
        chk("steamcmd", "SteamCMD trouvé (mises à jour)", steamcmd.exists() || steamcmd.components().count() == 1,
            steamcmd.display().to_string()),
    ];
    match s.load_world_options() {
        Err(e) => v.push(chk("ini", "PalWorldSettings.ini lisible", false, e.to_string())),
        Ok((o, from_default)) => {
            let detail = if from_default { format!("{} est vide : valeurs par défaut utilisées (enregistrez la configuration pour le remplir)", ini_path.display()) } else { ini_path.display().to_string() };
            v.push(chk("ini", "PalWorldSettings.ini lisible", true, detail));
            v.push(chk("rest_enabled", "API REST activée (RESTAPIEnabled=True)", ini::get(&o, "RESTAPIEnabled") == Some("True"), ""));
            let pw = ini::get(&o, "AdminPassword").unwrap_or("");
            v.push(chk("admin_password", "Mot de passe admin défini", !pw.is_empty(), ""));
            v.push(chk("password_match", "Mot de passe de l'application = celui du serveur", !pw.is_empty() && pw == s.rest.admin_password, ""));
            let port_ok = ini::get(&o, "RESTAPIPort").is_none_or(|p| p.parse::<u16>().ok() == Some(s.rest.port));
            v.push(chk("port_match", "Port REST de l'application = celui du serveur", port_ok, format!("app : {}", s.rest.port)));
        }
    }
    v
}

/// Fichiers + joignabilité de l'API REST.
pub async fn diagnose(s: &AppSettings) -> Vec<Check> {
    let mut v = diagnose_files(s);
    let reach = match RestClient::new(&s.rest) {
        Ok(api) => api.metrics().await.map(|_| String::new()).map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    v.push(chk("rest_reachable", "API REST joignable (serveur démarré)", reach.is_ok(), reach.err().unwrap_or_default()));
    v
}

/// Active l'API REST dans le .ini (RESTAPIEnabled, RESTAPIPort, AdminPassword), avec sauvegarde `.bak`,
/// et renvoie les paramètres de l'app alignés sur le serveur. Le serveur doit être redémarré ensuite.
pub fn fix_rest(s: &AppSettings, admin_password: &str) -> Result<AppSettings> {
    if admin_password.trim().is_empty() { return Err(Error::Other("mot de passe admin vide".into())); }
    let path = s.world_settings_path();
    let (mut o, _) = s.load_world_options()?;
    ini::set(&mut o, "RESTAPIEnabled", "True", false);
    ini::set(&mut o, "RESTAPIPort", &s.rest.port.to_string(), false);
    ini::set(&mut o, "AdminPassword", admin_password, true);
    if let Some(dir) = path.parent() { std::fs::create_dir_all(dir)?; }
    if path.exists() { std::fs::copy(&path, path.with_extension("ini.bak"))?; }
    std::fs::write(&path, ini::serialize(&o))?;
    let mut out = s.clone();
    out.rest.admin_password = admin_password.to_string();
    Ok(out)
}

/// Dossiers candidats contenant `PalServer.exe` (installations SteamCMD et bibliothèques Steam usuelles).
pub fn detect_server_dirs() -> Vec<std::path::PathBuf> {
    let mut roots: Vec<std::path::PathBuf> = Vec::new();
    for pf in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(v) = std::env::var_os(pf) { roots.push(std::path::PathBuf::from(v).join("Steam/steamapps/common")); }
    }
    for d in ["C", "D", "E", "F"] {
        let base = std::path::PathBuf::from(format!("{d}:/"));
        roots.push(base.join("SteamLibrary/steamapps/common"));
        roots.push(base.join("steamcmd/steamapps/common"));
        roots.push(base.join("Steam/steamapps/common"));
        roots.push(base.join("PalServer"));
    }
    if let Some(h) = std::env::var_os("USERPROFILE") {
        let h = std::path::PathBuf::from(h);
        roots.push(h.join("Desktop/steamcmd/steamapps/common"));
        roots.push(h.join("steamcmd/steamapps/common"));
    }
    let mut found = Vec::new();
    for r in roots {
        for cand in [r.clone(), r.join("PalServer")] {
            if cand.join("PalServer.exe").exists() && !found.contains(&cand) { found.push(cand); }
        }
    }
    found
}

/// Chemins usuels de `steamcmd.exe` présents sur la machine.
pub fn detect_steamcmd() -> Vec<std::path::PathBuf> {
    let mut v = Vec::new();
    for d in ["C", "D", "E"] {
        for sub in ["steamcmd", "SteamCMD"] {
            let p = std::path::PathBuf::from(format!("{d}:/{sub}/steamcmd.exe"));
            if p.exists() { v.push(p); }
        }
    }
    if let Some(h) = std::env::var_os("USERPROFILE") {
        let p = std::path::PathBuf::from(h).join("Desktop/steamcmd/steamcmd.exe");
        if p.exists() { v.push(p); }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(name: &str) -> AppSettings {
        let dir = std::env::temp_dir().join(format!("pal-setup-{name}-{}", std::process::id()));
        let cfg = dir.join("Pal/Saved/Config/WindowsServer");
        std::fs::create_dir_all(&cfg).unwrap();
        std::fs::write(cfg.join("PalWorldSettings.ini"), "[/Script/Pal.PalGameWorldSettings]\nOptionSettings=(ServerName=\"x\",RESTAPIEnabled=False)\n").unwrap();
        AppSettings { server_dir: dir, ..Default::default() }
    }

    #[test]
    fn detection_never_panics() {
        let _ = detect_server_dirs();
        let _ = detect_steamcmd();
    }

    #[test]
    fn diagnose_then_fix() {
        let s = setup("a");
        let failing = |v: &[Check]| v.iter().filter(|c| !c.ok).map(|c| c.id).collect::<Vec<_>>();
        let before = diagnose_files(&s);
        assert!(failing(&before).contains(&"rest_enabled") && failing(&before).contains(&"admin_password"));
        let s2 = fix_rest(&s, "secret").unwrap();
        let after = diagnose_files(&s2);
        assert!(!failing(&after).contains(&"rest_enabled"));
        assert!(!failing(&after).contains(&"password_match"));
        assert!(fix_rest(&s, " ").is_err());
        std::fs::remove_dir_all(&s.server_dir).ok();
    }
}
