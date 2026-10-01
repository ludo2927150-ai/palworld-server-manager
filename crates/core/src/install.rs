//! Installation en un clic : SteamCMD puis le serveur Palworld, depuis zéro.

use crate::{steamcmd, Error, Result};
use std::path::{Path, PathBuf};
use tokio::{io::AsyncReadExt, process::Command};

/// Adresse officielle de SteamCMD (Valve). Seule adresse téléchargée par ce module.
pub const STEAMCMD_URL: &str = "https://steamcdn-a.akamaihd.net/client/installer/steamcmd.zip";
/// Espace disque minimal conseillé pour le serveur (≈ 8 Go) plus de la marge.
pub const MIN_FREE_BYTES: u64 = 12 * 1_000_000_000;

/// Extrait `steamcmd.exe` de l'archive Valve dans `dest`.
pub fn extract_steamcmd(zip_bytes: &[u8], dest: &Path) -> Result<PathBuf> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes))?;
    std::fs::create_dir_all(dest)?;
    let mut f = zip.by_name("steamcmd.exe").map_err(|_| Error::Other("l'archive ne contient pas steamcmd.exe".into()))?;
    let out = dest.join("steamcmd.exe");
    std::io::copy(&mut f, &mut std::fs::File::create(&out)?)?;
    Ok(out)
}

pub async fn download_steamcmd(dest: &Path) -> Result<PathBuf> {
    let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(120)).build()?;
    let bytes = http.get(STEAMCMD_URL).send().await?.error_for_status()?.bytes().await?;
    if bytes.len() < 100_000 || &bytes[..2] != b"PK" { return Err(Error::Other("fichier SteamCMD téléchargé invalide".into())); }
    extract_steamcmd(&bytes, dest)
}

/// Découpe un flux de sortie en lignes sur `\n` comme sur `\r` (SteamCMD affiche sa progression en réécrivant la ligne).
pub fn split_progress(buf: &mut Vec<u8>, chunk: &[u8]) -> Vec<String> {
    buf.extend_from_slice(chunk);
    let mut out = Vec::new();
    while let Some(i) = buf.iter().position(|b| *b == b'\n' || *b == b'\r') {
        let line: Vec<u8> = buf.drain(..=i).collect();
        let s = String::from_utf8_lossy(&line).trim().to_string();
        if !s.is_empty() { out.push(s); }
    }
    out
}

/// Lance SteamCMD et transmet chaque ligne affichée à `on_line`. Renvoie le code de sortie.
pub async fn run_streaming(steamcmd_exe: &Path, args: &[String], mut on_line: impl FnMut(String)) -> Result<Option<i32>> {
    let mut child = Command::new(steamcmd_exe).args(args).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn()
        .map_err(|e| Error::Other(format!("SteamCMD introuvable ({}) : {e}", steamcmd_exe.display())))?;
    let mut out = child.stdout.take().ok_or_else(|| Error::Other("sortie de SteamCMD indisponible".into()))?;
    let (mut buf, mut chunk) = (Vec::new(), [0u8; 4096]);
    loop {
        let n = out.read(&mut chunk).await?;
        if n == 0 { break; }
        for l in split_progress(&mut buf, &chunk[..n]) { on_line(l); }
    }
    Ok(child.wait().await?.code())
}

/// Installe le serveur dans `server_dir`. Le succès est jugé sur la présence de `PalServer.exe`, le code de sortie de SteamCMD n'étant pas fiable.
pub async fn install_server(steamcmd_exe: &Path, server_dir: &Path, on_line: impl FnMut(String)) -> Result<()> {
    std::fs::create_dir_all(server_dir)?;
    run_streaming(steamcmd_exe, &steamcmd::args(server_dir), on_line).await?;
    if server_dir.join("PalServer.exe").exists() { Ok(()) } else { Err(Error::Other("l'installation s'est terminée sans PalServer.exe : relancez-la (SteamCMD s'est peut-être mis à jour pendant la première exécution)".into())) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn extracts_steamcmd_exe_only_and_rejects_other_archives() {
        let dir = std::env::temp_dir().join(format!("pal-inst-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut buf = Vec::new();
        {
            let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            z.start_file("steamcmd.exe", zip::write::FileOptions::default()).unwrap();
            z.write_all(b"MZ-fake").unwrap();
            z.finish().unwrap();
        }
        let exe = extract_steamcmd(&buf, &dir).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), b"MZ-fake");
        let mut other = Vec::new();
        { let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut other)); z.start_file("x.txt", zip::write::FileOptions::default()).unwrap(); z.finish().unwrap(); }
        assert!(extract_steamcmd(&other, &dir).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn progress_lines_split_on_carriage_returns_and_keep_partials() {
        let mut buf = Vec::new();
        assert_eq!(split_progress(&mut buf, b"Update state (0x61) downloading, progress: 1.00\rUpdate state (0x61) downloading, progress: 2.00\rpart"), ["Update state (0x61) downloading, progress: 1.00", "Update state (0x61) downloading, progress: 2.00"]);
        assert_eq!(split_progress(&mut buf, b"ial\r\n\r\n"), ["partial"]);
    }
}
