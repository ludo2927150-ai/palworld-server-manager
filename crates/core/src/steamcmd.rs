//! Installation / mise à jour du serveur via SteamCMD (app 2394010 = Palworld Dedicated Server).

use crate::{Error, Result};
use std::path::Path;
use tokio::process::Command;

pub const APP_ID: &str = "2394010";

pub fn args(server_dir: &Path) -> Vec<String> {
    ["+force_install_dir", &server_dir.to_string_lossy(), "+login", "anonymous", "+app_update", APP_ID, "validate", "+quit"]
        .iter().map(|s| s.to_string()).collect()
}

/// Lance SteamCMD (le serveur doit être arrêté). Renvoie les dernières lignes de sortie.
pub async fn update(steamcmd: &Path, server_dir: &Path) -> Result<String> {
    let out = Command::new(steamcmd).args(args(server_dir)).output().await
        .map_err(|e| Error::Other(format!("SteamCMD introuvable ({}) : {e}", steamcmd.display())))?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let tail = text.lines().rev().take(8).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
    if out.status.success() { Ok(tail) } else { Err(Error::Other(format!("SteamCMD a échoué ({}) :\n{tail}", out.status))) }
}

#[cfg(test)]
mod tests {
    #[test]
    fn builds_expected_args() {
        let a = super::args(std::path::Path::new("D:/pal"));
        assert_eq!(a[..2], ["+force_install_dir", "D:/pal"]);
        assert!(a.contains(&"2394010".to_string()) && a.last().unwrap() == "+quit");
    }
}
