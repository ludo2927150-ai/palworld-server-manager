//! Bot Discord : commandes slash (/statut, /joueurs, /sauvegarde, /demarrer, /arreter, /redemarrer, /annonce).
//!
//! Connexion sortante à la passerelle Discord (aucun port ouvert sur ce PC). Chaque commande est vérifiée contre la
//! liste d'identifiants Discord autorisés ; les actions de contrôle exigent en plus `allow_control`.

use crate::{remote::{Backend, Control}, settings::DiscordBotSettings, Error, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

const GATEWAY: &str = "wss://gateway.discord.gg/?v=10&encoding=json";
const API: &str = "https://discord.com/api/v10";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmd { Status, Players, Backup, Start, Stop, Restart, Announce }

impl Cmd {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "statut" => Cmd::Status, "joueurs" => Cmd::Players, "sauvegarde" => Cmd::Backup, "demarrer" => Cmd::Start,
            "arreter" => Cmd::Stop, "redemarrer" => Cmd::Restart, "annonce" => Cmd::Announce, _ => return None,
        })
    }
    fn is_control(self) -> bool { !matches!(self, Cmd::Status | Cmd::Players) }
}

/// Peut-on exécuter cette commande pour cet utilisateur ? `Err` = message à lui montrer.
pub fn authorize(cfg: &DiscordBotSettings, user_id: &str, cmd: Cmd) -> std::result::Result<(), &'static str> {
    if user_id.is_empty() || !cfg.allowed_user_ids.iter().any(|u| u == user_id) { return Err("⛔ Vous n'êtes pas autorisé à utiliser ce bot."); }
    if cmd.is_control() && !cfg.allow_control { return Err("⛔ Le contrôle du serveur est désactivé (lecture seule)."); }
    Ok(())
}

fn fmt_uptime(s: u64) -> String { format!("{} h {} min", s / 3600, (s % 3600) / 60) }

/// Exécute la commande et renvoie la réponse à afficher.
pub async fn run_command<B: Backend>(backend: &B, cmd: Cmd, arg: Option<&str>) -> String {
    let done = |r: Result<()>, ok: &str| match r { Ok(()) => format!("✅ {ok}"), Err(e) => format!("❌ {e}") };
    match cmd {
        Cmd::Status => {
            let s = backend.snapshot().await;
            if !s.running { return "🔴 Serveur arrêté.".into(); }
            match &s.metrics {
                Some(m) => format!("🟢 Serveur en marche — {}/{} joueurs, {} FPS, jour {}, en ligne depuis {}. CPU {:.0} %, RAM {:.1} Go.", m.currentplayernum, m.maxplayernum, m.serverfps, m.days, fmt_uptime(m.uptime), s.cpu_percent, s.memory_bytes as f64 / 1e9),
                None => format!("🟠 Processus en marche mais l'API REST ne répond pas. CPU {:.0} %, RAM {:.1} Go.", s.cpu_percent, s.memory_bytes as f64 / 1e9),
            }
        }
        Cmd::Players => {
            let s = backend.snapshot().await;
            if !s.running { "🔴 Serveur arrêté.".into() }
            else if s.players.is_empty() { "Personne en ligne.".into() }
            else { format!("👥 {} en ligne : {}", s.players.len(), s.players.iter().map(|p| format!("{} (niv. {})", p.name, p.level)).collect::<Vec<_>>().join(", ")) }
        }
        Cmd::Backup => match backend.backup_now().await { Ok(b) => format!("✅ Sauvegarde créée : {}", b.file_name), Err(e) => format!("❌ {e}") },
        Cmd::Start => done(backend.control(Control::Start).await, "Démarrage demandé."),
        Cmd::Stop => done(backend.control(Control::Stop).await, "Serveur arrêté (sauvegarde faite)."),
        Cmd::Restart => done(backend.control(Control::Restart).await, "Serveur redémarré."),
        Cmd::Announce => match arg.map(str::trim).filter(|a| !a.is_empty()) {
            Some(m) => done(backend.announce(m.chars().take(200).collect()).await, "Annonce envoyée."),
            None => "❌ Message vide.".into(),
        },
    }
}

fn command_definitions() -> Value {
    json!([
        {"name": "statut", "description": "État du serveur Palworld", "type": 1},
        {"name": "joueurs", "description": "Joueurs connectés", "type": 1},
        {"name": "sauvegarde", "description": "Créer une sauvegarde maintenant", "type": 1},
        {"name": "demarrer", "description": "Démarrer le serveur", "type": 1},
        {"name": "arreter", "description": "Arrêter le serveur (avec sauvegarde)", "type": 1},
        {"name": "redemarrer", "description": "Redémarrer le serveur (avec sauvegarde)", "type": 1},
        {"name": "annonce", "description": "Envoyer une annonce aux joueurs", "type": 1,
         "options": [{"name": "message", "description": "Texte de l'annonce", "type": 3, "required": true, "max_length": 200}]}
    ])
}

/// Ce que l'on retient d'une interaction Discord.
#[derive(Debug, PartialEq)]
pub struct Interaction { pub id: String, pub token: String, pub user_id: String, pub name: String, pub arg: Option<String> }

pub fn parse_interaction(d: &Value) -> Option<Interaction> {
    if d.get("type")?.as_u64()? != 2 { return None; } // commandes slash uniquement
    let user_id = d.pointer("/member/user/id").or_else(|| d.pointer("/user/id"))?.as_str()?.to_string();
    Some(Interaction {
        id: d.get("id")?.as_str()?.into(),
        token: d.get("token")?.as_str()?.into(),
        user_id,
        name: d.pointer("/data/name")?.as_str()?.into(),
        arg: d.pointer("/data/options/0/value").and_then(|v| v.as_str()).map(String::from),
    })
}

async fn respond<B: Backend>(http: &reqwest::Client, backend: &B, app_id: &str, cfg: &DiscordBotSettings, it: Interaction) {
    let ephemeral = 64;
    let reply = |content: String| json!({"content": content.chars().take(1900).collect::<String>(), "flags": ephemeral});
    let cb = format!("{API}/interactions/{}/{}/callback", it.id, it.token);
    let Some(cmd) = Cmd::parse(&it.name) else {
        let _ = http.post(&cb).json(&json!({"type": 4, "data": reply("❓ Commande inconnue.".into())})).send().await;
        return;
    };
    if let Err(msg) = authorize(cfg, &it.user_id, cmd) {
        let _ = http.post(&cb).json(&json!({"type": 4, "data": reply(msg.into())})).send().await;
        return;
    }
    backend.audit(&format!("discord:{}", it.user_id), &format!("/{}", it.name), it.arg.as_deref().unwrap_or(""));
    // Une action de contrôle peut durer plus que les 3 s accordées : accusé de réception, puis réponse différée.
    let _ = http.post(&cb).json(&json!({"type": 5, "data": {"flags": ephemeral}})).send().await;
    let text = run_command(backend, cmd, it.arg.as_deref()).await;
    let _ = http.patch(format!("{API}/webhooks/{app_id}/{}/messages/@original", it.token)).json(&reply(text)).send().await;
}

/// Une session de passerelle ; revient (Ok ou Err) quand la connexion tombe ou que Discord demande de se reconnecter.
async fn session<B: Backend>(backend: &B, cfg: &DiscordBotSettings, http: &reqwest::Client) -> Result<()> {
    let (mut ws, _) = tokio_tungstenite::connect_async(GATEWAY).await.map_err(|e| Error::Other(format!("passerelle Discord : {e}")))?;
    let mut seq: Option<u64> = None;
    let mut beat: Option<tokio::time::Interval> = None;
    let mut app_id = String::new();
    loop {
        let msg = tokio::select! {
            m = ws.next() => m,
            _ = async { match beat.as_mut() { Some(b) => { b.tick().await; } None => std::future::pending::<()>().await } } => {
                ws.send(Message::Text(json!({"op": 1, "d": seq}).to_string())).await.map_err(|e| Error::Other(e.to_string()))?;
                continue;
            }
        };
        let Some(Ok(Message::Text(txt))) = msg else {
            return match msg { Some(Ok(Message::Close(_))) | None => Ok(()), Some(Ok(_)) => continue, Some(Err(e)) => Err(Error::Other(e.to_string())) };
        };
        let v: Value = serde_json::from_str(&txt)?;
        if let Some(s) = v.get("s").and_then(|s| s.as_u64()) { seq = Some(s); }
        match v.get("op").and_then(|o| o.as_u64()) {
            Some(10) => {
                let ms = v.pointer("/d/heartbeat_interval").and_then(|x| x.as_u64()).unwrap_or(41250);
                let mut i = tokio::time::interval(Duration::from_millis(ms));
                i.tick().await;
                beat = Some(i);
                ws.send(Message::Text(json!({"op": 2, "d": {"token": cfg.bot_token, "intents": 0,
                    "properties": {"os": "windows", "browser": "palworld-manager", "device": "palworld-manager"}}}).to_string()))
                    .await.map_err(|e| Error::Other(e.to_string()))?;
            }
            Some(7) | Some(9) => return Ok(()), // reconnexion demandée / session invalide
            Some(0) => match v.get("t").and_then(|t| t.as_str()) {
                Some("READY") => {
                    app_id = v.pointer("/d/application/id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                    let url = format!("{API}/applications/{app_id}/commands");
                    let r = http.put(url).header("Authorization", format!("Bot {}", cfg.bot_token)).json(&command_definitions()).send().await
                        .and_then(|r| r.error_for_status());
                    if let Err(e) = r { eprintln!("discord : enregistrement des commandes : {e}"); }
                }
                Some("INTERACTION_CREATE") => {
                    if let Some(it) = v.get("d").and_then(parse_interaction) {
                        let (b, c, h, a) = (backend.clone(), cfg.clone(), http.clone(), app_id.clone());
                        tokio::spawn(async move { respond(&h, &b, &a, &c, it).await });
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

/// Boucle du bot ; ne s'arrête que lorsque la tâche est annulée. Reconnexion automatique avec attente croissante.
/// Un jeton refusé (code 4004) n'est pas réessayé en boucle serrée : l'attente maximale est de 5 minutes.
pub async fn run<B: Backend>(backend: B, cfg: DiscordBotSettings) {
    let http = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().expect("client http");
    let mut wait = 5u64;
    loop {
        let started = std::time::Instant::now();
        if let Err(e) = session(&backend, &cfg, &http).await { eprintln!("discord : {e}"); }
        wait = if started.elapsed() > Duration::from_secs(120) { 5 } else { (wait * 2).min(300) };
        tokio::time::sleep(Duration::from_secs(wait)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{backup::BackupInfo, history::{Sample, Session}, logs::LogChunk, monitor::Snapshot};
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Fake(Arc<Mutex<Vec<String>>>);
    impl Backend for Fake {
        async fn snapshot(&self) -> Snapshot { Snapshot { running: true, ..Default::default() } }
        async fn control(&self, a: Control) -> Result<()> { self.0.lock().unwrap().push(format!("{a:?}")); Ok(()) }
        async fn history(&self, _: i64) -> Vec<Sample> { vec![] }
        async fn sessions(&self, _: i64) -> Vec<Session> { vec![] }
        async fn logs(&self, _: Option<u64>) -> Result<LogChunk> { Err(Error::Other("x".into())) }
        async fn backup_now(&self) -> Result<BackupInfo> { Err(Error::Other("disque plein".into())) }
        async fn announce(&self, m: String) -> Result<()> { self.0.lock().unwrap().push(format!("announce:{m}")); Ok(()) }
        async fn kick(&self, _: String) -> Result<()> { Ok(()) }
    }

    fn cfg(control: bool) -> DiscordBotSettings {
        DiscordBotSettings { enabled: true, bot_token: "t".into(), allowed_user_ids: vec!["42".into()], allow_control: control }
    }

    #[test]
    fn authorization_rules() {
        assert!(authorize(&cfg(false), "42", Cmd::Status).is_ok());
        assert!(authorize(&cfg(false), "42", Cmd::Restart).is_err()); // lecture seule
        assert!(authorize(&cfg(true), "42", Cmd::Restart).is_ok());
        assert!(authorize(&cfg(true), "7", Cmd::Status).is_err()); // inconnu
        assert!(authorize(&cfg(true), "", Cmd::Status).is_err());
        let empty = DiscordBotSettings { allowed_user_ids: vec![], ..cfg(true) };
        assert!(authorize(&empty, "42", Cmd::Status).is_err()); // liste vide = personne
    }

    #[test]
    fn parses_slash_command_interactions_only() {
        let d = json!({"id": "1", "token": "tok", "type": 2, "member": {"user": {"id": "42"}}, "data": {"name": "annonce", "options": [{"name": "message", "value": "Salut"}]}});
        let it = parse_interaction(&d).unwrap();
        assert_eq!((it.user_id.as_str(), it.name.as_str(), it.arg.as_deref()), ("42", "annonce", Some("Salut")));
        assert!(parse_interaction(&json!({"id": "1", "token": "t", "type": 3, "user": {"id": "1"}, "data": {"name": "x"}})).is_none());
        let dm = json!({"id": "1", "token": "tok", "type": 2, "user": {"id": "9"}, "data": {"name": "statut"}});
        assert_eq!(parse_interaction(&dm).unwrap().user_id, "9");
    }

    #[tokio::test]
    async fn commands_reach_the_backend() {
        let f = Fake::default();
        assert!(run_command(&f, Cmd::Restart, None).await.starts_with('✅'));
        assert!(run_command(&f, Cmd::Announce, Some("  ")).await.starts_with('❌'));
        assert!(run_command(&f, Cmd::Announce, Some("Salut")).await.starts_with('✅'));
        assert!(run_command(&f, Cmd::Backup, None).await.contains("disque plein"));
        assert_eq!(*f.0.lock().unwrap(), vec!["Restart".to_string(), "announce:Salut".to_string()]);
        assert!(command_definitions().as_array().unwrap().len() == 7);
    }
}
