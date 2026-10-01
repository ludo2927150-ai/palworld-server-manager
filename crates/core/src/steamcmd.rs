//! Installation / mise à jour du serveur via SteamCMD (app 2394010 = Palworld Dedicated Server).

use crate::{Error, Result};
use std::path::Path;
use tokio::process::Command;

pub const APP_ID: &str = "2394010";

pub fn args(server_dir: &Path) -> Vec<String> {
    ["+force_install_dir", &server_dir.to_string_lossy(), "+login", "anonymous", "+app_update", APP_ID, "validate", "+quit"]
        .iter().map(|s| s.to_string()).collect()
}

/// Arguments pour télécharger un élément du Workshop de Palworld dans `<serveur>\steamapps\workshop\content\1623730\<id>`.
/// `force_install_dir` doit précéder la connexion.
pub fn workshop_args(server_dir: &Path, workshop_id: &str) -> Vec<String> {
    ["+force_install_dir", &server_dir.to_string_lossy(), "+login", "anonymous", "+workshop_download_item", crate::mods::APP_ID, workshop_id, "validate", "+quit"]
        .iter().map(|s| s.to_string()).collect()
}

/// Télécharge un mod du Workshop. Le code de sortie de SteamCMD n'étant pas fiable, on lit son message de fin.
pub async fn download_workshop(steamcmd: &Path, server_dir: &Path, workshop_id: &str) -> Result<()> {
    let out = Command::new(steamcmd).args(workshop_args(server_dir, workshop_id)).output().await
        .map_err(|e| Error::Other(format!("SteamCMD introuvable ({}) : {e}", steamcmd.display())))?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    if text.contains("Success. Downloaded item") { return Ok(()); }
    let tail = text.lines().rev().take(6).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
    let hint = if text.contains("Access Denied") || text.contains("No subscription") || text.contains("Failure") {
        "\nSteam refuse le téléchargement anonyme pour ce mod. Abonnez-vous-y depuis le client Steam (sur ce PC), puis choisissez le dossier Workshop de Steam."
    } else { "" };
    Err(Error::Other(format!("téléchargement du mod {workshop_id} échoué :\n{tail}{hint}")))
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
    fn workshop_args_put_install_dir_before_login() {
        let a = super::workshop_args(std::path::Path::new("D:/pal"), "3123456789");
        assert_eq!(a[..2], ["+force_install_dir", "D:/pal"]);
        let (login, dl) = (a.iter().position(|x| x == "+login").unwrap(), a.iter().position(|x| x == "+workshop_download_item").unwrap());
        assert!(2 <= login && login < dl);
        assert_eq!(a[dl + 1..dl + 3], ["1623730", "3123456789"]);
    }

    #[test]
    fn builds_expected_args() {
        let a = super::args(std::path::Path::new("D:/pal"));
        assert_eq!(a[..2], ["+force_install_dir", "D:/pal"]);
        assert!(a.contains(&"2394010".to_string()) && a.last().unwrap() == "+quit");
    }
}
