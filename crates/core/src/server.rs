//! Cycle de vie de `PalServer.exe`. Le serveur est identifié par nom de processus : `PalServer.exe`
//! n'est qu'un lanceur qui engendre `PalServer-Win64-Shipping-Cmd.exe`, et un serveur lancé avant
//! l'application (ou en dehors d'elle) est ainsi « adopté » automatiquement.

use crate::{backup, rest::RestClient, settings::AppSettings, Error, Result};
use std::time::Duration;
use sysinfo::System;
use std::{path::{Path, PathBuf}, process::Stdio};
use tokio::{io::{AsyncBufReadExt, AsyncWriteExt, BufReader}, process::{Child, Command}, sync::Mutex};

/// PID de tous les processus du serveur (lanceur + jeu).
pub fn find_server_pids() -> Vec<u32> {
    let mut sys = System::new();
    sys.refresh_processes();
    sys.processes().iter()
        .filter(|(_, p)| p.name().to_ascii_lowercase().starts_with("palserver"))
        .map(|(pid, _)| pid.as_u32()).collect()
}

/// Dossier d'installation du serveur actuellement en cours d'exécution (déduit du chemin de son processus).
pub fn running_server_dir() -> Option<PathBuf> {
    let mut sys = System::new();
    sys.refresh_processes();
    sys.processes().values().filter(|p| p.name().to_ascii_lowercase().starts_with("palserver")).find_map(|p| install_dir_from_exe(p.exe()?))
}

/// `…\PalServer\PalServer.exe` → `…\PalServer` ; `…\PalServer\Pal\Binaries\Win64\PalServer-Win64-Shipping-Cmd.exe` → `…\PalServer`.
pub fn install_dir_from_exe(exe: &Path) -> Option<PathBuf> {
    let name = exe.file_name()?.to_string_lossy().to_ascii_lowercase();
    if name == "palserver.exe" { return exe.parent().map(Path::to_path_buf); }
    if name.starts_with("palserver-") {
        let dir = exe.ancestors().nth(4)?; // exe → Win64 → Binaries → Pal → racine
        let in_pal = exe.ancestors().nth(3).and_then(|p| p.file_name()).is_some_and(|n| n.eq_ignore_ascii_case("Pal"));
        return in_pal.then(|| dir.to_path_buf());
    }
    None
}

/// Deux chemins désignent-ils le même dossier ? (insensible à la casse et aux séparateurs)
pub fn same_dir(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()).to_string_lossy().to_ascii_lowercase().replace('\\', "/").trim_end_matches('/').to_string();
    norm(a) == norm(b)
}

fn kill_all() {
    let mut sys = System::new();
    sys.refresh_processes();
    for p in sys.processes().values().filter(|p| p.name().to_ascii_lowercase().starts_with("palserver")) { p.kill(); }
}

#[derive(Default)]
pub struct ServerController {
    child: Mutex<Option<Child>>,
    /// Dernière sauvegarde d'arrêt qui a échoué (récupérée une fois par le superviseur pour alerter).
    backup_warning: std::sync::Mutex<Option<String>>,
    /// Fichier où la sortie console du serveur (lancé par l'application) est enregistrée.
    console_log: Option<PathBuf>,
    pty: std::sync::Mutex<Option<PtyHandle>>,
}

const CONSOLE_LOG_MAX: u64 = 5 * 1024 * 1024;

/// Tâche qui écrit les lignes reçues dans `log` (l'ancien fichier devient `.old` ; rotation à 5 Mo). À appeler depuis le runtime tokio.
fn spawn_log_writer(log: PathBuf) -> tokio::sync::mpsc::UnboundedSender<String> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    tokio::spawn(async move {
        if let Some(dir) = log.parent() { let _ = tokio::fs::create_dir_all(dir).await; }
        let _ = tokio::fs::rename(&log, log.with_extension("old")).await;
        let Ok(mut f) = tokio::fs::File::create(&log).await else { return };
        let mut size = 0u64;
        while let Some(line) = rx.recv().await {
            if size > CONSOLE_LOG_MAX {
                let _ = tokio::fs::rename(&log, log.with_extension("old")).await;
                match tokio::fs::File::create(&log).await { Ok(n) => { f = n; size = 0; } Err(_) => return }
            }
            let buf = format!("{line}\n");
            if f.write_all(buf.as_bytes()).await.is_err() { return; }
            let _ = f.flush().await;
            size += buf.len() as u64;
        }
    });
    tx
}

/// Recopie stdout + stderr du processus dans `log`. Repli : les programmes qui écrivent dans un tuyau le font souvent par blocs
/// (sortie bufferisée), d'où l'usage préféré d'une console virtuelle (`spawn_pty`).
fn capture_output(child: &mut Child, log: PathBuf) {
    let tx = spawn_log_writer(log);
    if let Some(o) = child.stdout.take() {
        let tx = tx.clone();
        tokio::spawn(async move { let mut l = BufReader::new(o).lines(); while let Ok(Some(line)) = l.next_line().await { if tx.send(line).is_err() { break; } } });
    }
    if let Some(e) = child.stderr.take() {
        tokio::spawn(async move { let mut l = BufReader::new(e).lines(); while let Ok(Some(line)) = l.next_line().await { if tx.send(line).is_err() { break; } } });
    }
}

/// Retire les séquences d'échappement de terminal (couleurs, curseur, titre de fenêtre) et les caractères de contrôle.
pub fn clean_terminal_line(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\x1b' => match it.peek() {
                Some('[') => { it.next(); while let Some(&n) = it.peek() { it.next(); if ('@'..='~').contains(&n) { break; } } }
                Some(']') => { it.next(); while let Some(n) = it.next() { if n == '\x07' { break; } else if n == '\x1b' { it.next(); break; } } }
                Some('(' | ')') => { it.next(); it.next(); }
                _ => { it.next(); }
            },
            '\t' => out.push(' '),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out.trim_end().to_string()
}

/// Console virtuelle du serveur (ConPTY sous Windows) : le programme croit parler à un vrai terminal, donc sa sortie est
/// immédiate et complète, et aucune fenêtre n'apparaît. Il faut garder `_master` vivant : le fermer arrête le serveur.
struct PtyHandle {
    _master: Box<dyn portable_pty::MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

fn spawn_pty(exe: &Path, args: &[String], cwd: &Path, log: PathBuf) -> Result<PtyHandle> {
    fn err<E: std::fmt::Display>(e: E) -> Error { Error::Other(format!("console virtuelle : {e}")) }
    let pair = portable_pty::native_pty_system().openpty(portable_pty::PtySize { rows: 50, cols: 220, pixel_width: 0, pixel_height: 0 }).map_err(err)?;
    let mut cmd = portable_pty::CommandBuilder::new(exe);
    cmd.args(args);
    cmd.cwd(cwd);
    let child = pair.slave.spawn_command(cmd).map_err(err)?;
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().map_err(err)?;
    let tx = spawn_log_writer(log);
    std::thread::spawn(move || {
        use std::io::Read;
        let (mut buf, mut pending) = ([0u8; 4096], Vec::<u8>::new());
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    pending.extend_from_slice(&buf[..n]);
                    while let Some(i) = pending.iter().position(|b| *b == b'\n') {
                        let line: Vec<u8> = pending.drain(..=i).collect();
                        let text = clean_terminal_line(&String::from_utf8_lossy(&line));
                        if !text.is_empty() && tx.send(text).is_err() { return; }
                    }
                }
            }
        }
    });
    Ok(PtyHandle { _master: pair.master, child })
}

impl ServerController {
    pub fn new() -> Self { Self::default() }

    pub fn with_console_log(mut self, path: PathBuf) -> Self { self.console_log = Some(path); self }
    pub fn console_log_path(&self) -> Option<&Path> { self.console_log.as_deref() }

    pub async fn is_running(&self) -> bool {
        if let Ok(mut p) = self.pty.lock() {
            if let Some(h) = p.as_mut() {
                if matches!(h.child.try_wait(), Ok(None)) { return true; }
                *p = None; // sorti : on libère la console virtuelle
            }
        }
        let mut g = self.child.lock().await;
        if let Some(c) = g.as_mut() {
            if !matches!(c.try_wait(), Ok(Some(_))) { return true; }
            *g = None;
        }
        !find_server_pids().is_empty()
    }

    pub fn take_backup_warning(&self) -> Option<String> { self.backup_warning.lock().ok().and_then(|mut g| g.take()) }

    pub fn pid(&self) -> Option<u32> {
        if let Some(pid) = self.pty.lock().ok().and_then(|p| p.as_ref().and_then(|h| h.child.process_id())) { return Some(pid); }
        self.child.try_lock().ok().and_then(|g| g.as_ref().and_then(|c| c.id())).or_else(|| find_server_pids().first().copied())
    }

    pub async fn start(&self, s: &AppSettings) -> Result<()> {
        if self.is_running().await { return Err(Error::Other("le serveur tourne déjà".into())); }
        let exe = s.exe_path();
        if !exe.exists() { return Err(Error::Other(format!("introuvable : {}", exe.display()))); }
        // Console capturée : on démarre le moteur directement. Le lanceur PalServer.exe ouvre sinon sa propre fenêtre
        // pour le moteur, hors de portée de l'application. Si le moteur s'arrête aussitôt, repli sur le lanceur.
        let game = s.game_exe_path();
        if let (true, Some(log), true) = (s.capture_console, &self.console_log, game.exists()) {
            let mut pty_args = vec!["Pal".to_string()];
            pty_args.extend(s.launch_args.iter().cloned());
            if let Ok(mut h) = spawn_pty(&game, &pty_args, &s.server_dir, log.clone()) {
                tokio::time::sleep(Duration::from_secs(3)).await;
                if matches!(h.child.try_wait(), Ok(None)) {
                    if let Ok(mut p) = self.pty.lock() { *p = Some(h); }
                    return Ok(());
                }
            }
            let mut cmd = Command::new(&game);
            cmd.arg("Pal").args(&s.launch_args).current_dir(&s.server_dir).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
            #[cfg(windows)]
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
            if let Ok(mut child) = cmd.spawn() {
                tokio::time::sleep(Duration::from_secs(3)).await;
                if matches!(child.try_wait(), Ok(None)) {
                    capture_output(&mut child, log.clone());
                    *self.child.lock().await = Some(child);
                    return Ok(());
                }
            }
        }
        let child = Command::new(exe).args(&s.launch_args).current_dir(&s.server_dir).spawn()?;
        *self.child.lock().await = Some(child);
        Ok(())
    }

    /// Arrêt propre via l'API REST (sauvegarde + décompte), repli sur kill après `grace`.
    /// Une fois le serveur réellement arrêté (monde figé), une sauvegarde est faite si `backup.on_stop` est activé :
    /// tous les chemins (bouton, horaires, mise à jour, redémarrage) passent donc par ici.
    pub async fn stop(&self, s: &AppSettings, grace: Duration) -> Result<()> {
        if !self.is_running().await { return Ok(()); }
        if let Ok(api) = RestClient::new(&s.rest) {
            let _ = api.shutdown(10, "Arrêt du serveur dans 10 secondes").await;
        }
        let deadline = tokio::time::Instant::now() + grace;
        let mut stopped = false;
        while tokio::time::Instant::now() < deadline {
            if !self.is_running().await { stopped = true; break; }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        if !stopped {
            kill_all();
            *self.child.lock().await = None;
            tokio::time::sleep(Duration::from_secs(2)).await; // laisse Windows libérer les fichiers
        }
        if s.backup.on_stop {
            let s2 = s.clone();
            let label = if stopped { "arret" } else { "arret-force" };
            let res = tokio::task::spawn_blocking(move || backup::backup_all(&s2, Some(label))).await;
            let err = match res { Ok(Ok(_)) => None, Ok(Err(e)) => Some(e.to_string()), Err(e) => Some(e.to_string()) };
            if let (Some(e), Ok(mut g)) = (err, self.backup_warning.lock()) { *g = Some(e); }
        }
        Ok(())
    }

    pub async fn restart(&self, s: &AppSettings) -> Result<()> {
        self.stop(s, Duration::from_secs(60)).await?;
        self.start(s).await
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_sequences_are_stripped() {
        assert_eq!(clean_terminal_line("\x1b[2J\x1b[H\x1b]0;titre\x07[LOG] \x1b[32mKilz\x1b[0m joined\r\n"), "[LOG] Kilz joined");
        assert_eq!(clean_terminal_line("\x1b[?25l\x1b[1;1H\x1b[K"), "");
        assert_eq!(clean_terminal_line("a\tb"), "a b");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn pty_gives_unbuffered_complete_output() {
        let dir = std::env::temp_dir().join(format!("pal-pty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("console.log");
        // printf sans retour à la ligne final puis sleep : un tuyau bufferiserait ; une console virtuelle non.
        let mut h = spawn_pty(Path::new("/bin/sh"), &["-c".into(), "printf 'ligne un\\nligne deux\\n'; sleep 1".into()], &dir, log.clone()).unwrap();
        for _ in 0..40 { if std::fs::read_to_string(&log).map(|s| s.lines().count() >= 2).unwrap_or(false) { break; } tokio::time::sleep(Duration::from_millis(50)).await; }
        assert!(matches!(h.child.try_wait(), Ok(None)), "le programme tourne encore pendant qu'on lit sa sortie");
        assert_eq!(std::fs::read_to_string(&log).unwrap().lines().collect::<Vec<_>>(), ["ligne un", "ligne deux"]);
        let _ = h.child.kill();
        std::fs::remove_dir_all(dir).ok();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn console_output_is_captured_and_old_log_kept() {
        let dir = std::env::temp_dir().join(format!("pal-con-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("console.log");
        std::fs::write(&log, "ancien\n").unwrap();
        let mut c = Command::new("sh").args(["-c", "echo '[LOG] Kilz joined'; echo oups >&2"]).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        capture_output(&mut c, log.clone());
        c.wait().await.unwrap();
        for _ in 0..50 { if std::fs::read_to_string(&log).map(|s| s.lines().count() >= 2).unwrap_or(false) { break; } tokio::time::sleep(Duration::from_millis(50)).await; }
        let text = std::fs::read_to_string(&log).unwrap();
        assert!(text.contains("[LOG] Kilz joined") && text.contains("oups"), "{text}");
        assert_eq!(std::fs::read_to_string(log.with_extension("old")).unwrap(), "ancien\n");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn install_dir_is_derived_from_process_paths() {
        assert_eq!(install_dir_from_exe(Path::new("D:/Games/PalServer/PalServer.exe")).unwrap(), Path::new("D:/Games/PalServer"));
        assert_eq!(install_dir_from_exe(Path::new("D:/Games/PalServer/Pal/Binaries/Win64/PalServer-Win64-Shipping-Cmd.exe")).unwrap(), Path::new("D:/Games/PalServer"));
        assert!(install_dir_from_exe(Path::new("D:/x/Other/Win64/PalServer-Win64-Shipping-Cmd.exe")).is_none());
        assert!(install_dir_from_exe(Path::new("D:/x/notepad.exe")).is_none());
        assert!(same_dir(Path::new("D:/Games/PalServer/"), Path::new("D:/Games/PalServer")));
        assert!(!same_dir(Path::new("D:/a"), Path::new("D:/b")));
    }
}
