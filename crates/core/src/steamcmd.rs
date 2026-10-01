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


/// Numéro de build installé, lu dans `steamapps/appmanifest_2394010.acf`.
pub fn installed_build(server_dir: &Path) -> Option<String> {
    let acf = std::fs::read_to_string(server_dir.join(format!("steamapps/appmanifest_{APP_ID}.acf"))).ok()?;
    quoted_value_after(&acf, "buildid")
}

/// Valeur du premier `"clé"   "valeur"` à partir de `from` dans un texte VDF.
fn quoted_value_after(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let i = text.find(&needle)? + needle.len();
    let rest = text[i..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let v = &rest[..rest.find('"')?];
    (!v.is_empty() && v.chars().all(|c| c.is_ascii_digit())).then(|| v.to_string())
}

/// Extrait le build de la branche publique d'une sortie `app_info_print`.
pub fn parse_public_build(app_info: &str) -> Option<String> {
    let b = app_info.find("\"branches\"")?;
    let after = &app_info[b..];
    let p = after.find("\"public\"")?;
    quoted_value_after(&after[p..], "buildid")
}

/// Build public actuel sur Steam (connexion anonyme).
pub async fn latest_build(steamcmd: &Path) -> Result<String> {
    let out = Command::new(steamcmd)
        .args(["+login", "anonymous", "+app_info_update", "1", "+app_info_print", APP_ID, "+quit"])
        .output().await
        .map_err(|e| Error::Other(format!("SteamCMD introuvable ({}) : {e}", steamcmd.display())))?;
    parse_public_build(&String::from_utf8_lossy(&out.stdout)).ok_or_else(|| Error::Other("build Steam illisible (SteamCMD n'a pas renvoyé les informations de l'application)".into()))
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
    fn parses_manifest_and_app_info() {
        let dir = std::env::temp_dir().join(format!("pal-acf-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("steamapps")).unwrap();
        std::fs::write(dir.join("steamapps/appmanifest_2394010.acf"), "\"AppState\"\n{\n\t\"appid\"\t\t\"2394010\"\n\t\"buildid\"\t\t\"15012345\"\n}").unwrap();
        assert_eq!(super::installed_build(&dir).as_deref(), Some("15012345"));
        std::fs::remove_dir_all(&dir).ok();
        let info = "\"2394010\"\n{\n\"depots\"\n{\n\"branches\"\n{\n\t\"beta\"\n\t{\n\t\t\"buildid\"\t\"111\"\n\t}\n\t\"public\"\n\t{\n\t\t\"buildid\"\t\"15099999\"\n\t\t\"timeupdated\"\t\"1\"\n\t}\n}\n}\n}";
        assert_eq!(super::parse_public_build(info).as_deref(), Some("15099999"));
        assert!(super::parse_public_build("rien").is_none());
    }

    #[test]
    fn builds_expected_args() {
        let a = super::args(std::path::Path::new("D:/pal"));
        assert_eq!(a[..2], ["+force_install_dir", "D:/pal"]);
        assert!(a.contains(&"2394010".to_string()) && a.last().unwrap() == "+quit");
    }
}
