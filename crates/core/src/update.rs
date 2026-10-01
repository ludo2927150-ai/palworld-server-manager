//! Mises à jour de l'application via les Releases GitHub (aucune donnée envoyée, simple lecture publique).

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub const REPO: &str = "ludo2927150-ai/palworld-server-manager";
const DOWNLOAD_PREFIX: &str = "https://github.com/ludo2927150-ai/palworld-server-manager/releases/download/";

/// `1.2.3` (stable) ou `1.2.3-test.N` (version de test) ; une version stable est plus récente que ses tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version { major: u32, minor: u32, patch: u32, test: Option<u32> }

impl Version {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('v');
        let (core, test) = match s.split_once('-') {
            Some((c, pre)) => (c, Some(pre.strip_prefix("test.")?.parse().ok()?)),
            None => (s, None),
        };
        let mut it = core.split('.').map(|p| p.parse::<u32>().ok());
        let v = Self { major: it.next()??, minor: it.next()??, patch: it.next()??, test };
        it.next().is_none().then_some(v)
    }
}
impl Ord for Version {
    fn cmp(&self, o: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(o.major, o.minor, o.patch)).then_with(|| match (self.test, o.test) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(a), Some(b)) => a.cmp(&b),
        })
    }
}
impl PartialOrd for Version { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }

/// Version de cette application : `PALMANAGER_BUILD` (numéro de build CI) la marque comme version de test.
pub fn current_version_string() -> String {
    match option_env!("PALMANAGER_BUILD") {
        Some(n) if !n.is_empty() => format!("{}-test.{n}", env!("CARGO_PKG_VERSION")),
        _ => env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[derive(Debug, Deserialize)]
struct GhAsset { name: String, browser_download_url: String }
#[derive(Debug, Deserialize)]
struct GhRelease { tag_name: String, #[serde(default)] draft: bool, #[serde(default)] body: Option<String>, #[serde(default)] assets: Vec<GhAsset> }

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct UpdateInfo { pub current: String, pub latest: String, pub notes: String, pub url: String, pub asset: String }

/// Plus récente version strictement supérieure à `current` ayant un installeur Windows valide.
pub fn pick_update(current: &str, releases_json: &str) -> Result<Option<UpdateInfo>> {
    let cur = Version::parse(current).ok_or_else(|| Error::Other(format!("version courante illisible : {current}")))?;
    let releases: Vec<GhRelease> = serde_json::from_str(releases_json)?;
    let best = releases.into_iter()
        .filter(|r| !r.draft)
        .filter_map(|r| Some((Version::parse(&r.tag_name)?, r)))
        .filter(|(v, _)| *v > cur)
        .filter_map(|(v, r)| {
            let a = r.assets.iter().find(|a| a.name.ends_with("_x64-setup.exe") && a.browser_download_url.starts_with(DOWNLOAD_PREFIX))?;
            Some((v, UpdateInfo { current: current.into(), latest: r.tag_name.trim_start_matches('v').into(), notes: r.body.clone().unwrap_or_default(), url: a.browser_download_url.clone(), asset: a.name.clone() }))
        })
        .max_by_key(|(v, _)| *v);
    Ok(best.map(|(_, i)| i))
}

fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder().user_agent("palworld-server-manager").timeout(std::time::Duration::from_secs(60)).build()?)
}

pub async fn check() -> Result<Option<UpdateInfo>> {
    let body = client()?.get(format!("https://api.github.com/repos/{REPO}/releases?per_page=30"))
        .header("Accept", "application/vnd.github+json").send().await?.error_for_status()?.text().await?;
    pick_update(&current_version_string(), &body)
}

/// Télécharge l'installeur dans le dossier temporaire et renvoie son chemin. N'accepte que les URL de release de ce dépôt.
pub async fn download(info: &UpdateInfo) -> Result<std::path::PathBuf> {
    if !info.url.starts_with(DOWNLOAD_PREFIX) || info.asset.contains(['/', '\\']) || !info.asset.ends_with(".exe") {
        return Err(Error::Other("adresse de mise à jour refusée".into()));
    }
    let bytes = client()?.get(&info.url).send().await?.error_for_status()?.bytes().await?;
    if bytes.len() < 1_000_000 || &bytes[..2] != b"MZ" { return Err(Error::Other("fichier téléchargé invalide".into())); }
    let path = std::env::temp_dir().join(&info.asset);
    std::fs::write(&path, &bytes)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_ordering() {
        let v = |s| Version::parse(s).unwrap();
        assert!(v("0.1.0-test.10") > v("0.1.0-test.9"));
        assert!(v("0.1.0") > v("0.1.0-test.99"));
        assert!(v("v0.2.0-test.1") > v("0.1.0"));
        assert!(Version::parse("0.1").is_none() && Version::parse("0.1.0-beta").is_none() && Version::parse("a.b.c").is_none());
    }

    fn release(tag: &str, asset_url: &str) -> String {
        format!(r#"{{"tag_name":"{tag}","draft":false,"body":"notes","assets":[{{"name":"Palworld_x64-setup.exe","browser_download_url":"{asset_url}"}}]}}"#)
    }

    #[test]
    fn picks_newest_valid_installer_only() {
        let good = |t: &str| format!("{DOWNLOAD_PREFIX}{t}/Palworld_x64-setup.exe");
        let json = format!("[{},{},{},{}]", release("v0.1.0-test.3", &good("v0.1.0-test.3")), release("v0.1.0-test.5", &good("v0.1.0-test.5")),
            release("v0.1.0-test.9", "https://evil.example/x_x64-setup.exe"), release("v0.1.0-test.1", &good("v0.1.0-test.1")));
        let u = pick_update("0.1.0-test.2", &json).unwrap().unwrap();
        assert_eq!(u.latest, "0.1.0-test.5"); // test.9 écartée : hôte non autorisé
        assert!(pick_update("0.1.0-test.5", &json).unwrap().is_none());
    }

    #[test]
    fn drafts_and_garbage_are_ignored() {
        let json = r#"[{"tag_name":"v9.9.9","draft":true,"assets":[]},{"tag_name":"nightly","assets":[]}]"#;
        assert!(pick_update("0.1.0", json).unwrap().is_none());
        assert!(pick_update("zzz", "[]").is_err());
    }
}
