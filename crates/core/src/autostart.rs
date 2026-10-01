//! Lancement avec Windows : clé `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (sans droits admin).
//! Implémentée via `reg.exe` pour obtenir des messages d'erreur lisibles ; l'appli démarre avec `--minimized`.

use crate::{Error, Result};
use std::path::Path;

pub const VALUE_NAME: &str = "PalworldServerManager";
#[cfg(windows)]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

/// Valeur de la clé : chemin de l'exécutable entre guillemets + argument de démarrage réduit.
pub fn run_value(exe: &Path) -> String {
    format!("\"{}\" --minimized", exe.display())
}

#[cfg(windows)]
fn reg(args: &[&str]) -> Result<std::process::Output> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("reg").args(args).creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output().map_err(|e| Error::Other(format!("reg.exe introuvable : {e}")))
}

#[cfg(windows)]
pub fn is_enabled() -> Result<bool> { Ok(reg(&["query", RUN_KEY, "/v", VALUE_NAME])?.status.success()) }

#[cfg(windows)]
pub fn set_enabled(exe: &Path, enabled: bool) -> Result<()> {
    let out = if enabled {
        reg(&["add", RUN_KEY, "/v", VALUE_NAME, "/t", "REG_SZ", "/d", &run_value(exe), "/f"])?
    } else {
        if !is_enabled()? { return Ok(()); } // rien à supprimer
        reg(&["delete", RUN_KEY, "/v", VALUE_NAME, "/f"])?
    };
    if out.status.success() { return Ok(()); }
    let msg = String::from_utf8_lossy(if out.stderr.is_empty() { &out.stdout } else { &out.stderr }).trim().to_string();
    Err(Error::Other(format!("modification du démarrage Windows refusée : {msg}")))
}

#[cfg(not(windows))]
pub fn is_enabled() -> Result<bool> { Ok(false) }

#[cfg(not(windows))]
pub fn set_enabled(_exe: &Path, _enabled: bool) -> Result<()> {
    Err(Error::Other("le lancement au démarrage n'est disponible que sous Windows".into()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn value_is_quoted_and_minimized() {
        let v = super::run_value(std::path::Path::new(r"C:\Program Files\Palworld Manager\app.exe"));
        assert_eq!(v, r#""C:\Program Files\Palworld Manager\app.exe" --minimized"#);
    }
}
