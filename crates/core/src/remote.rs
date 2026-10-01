//! Accès à distance : API JSON + page mobile, protégées par un jeton. Le routeur est générique sur
//! un `Backend` (implémenté par la couche Tauri) : il est donc testable sans Tauri.
//!
//! Volontairement exposé : état, historique, journal, démarrer/arrêter/redémarrer, sauvegarde, annonce,
//! expulsion. Volontairement NON exposé : paramètres de l'application (secrets), écriture de la
//! configuration du monde, restauration de sauvegarde, mise à jour SteamCMD, bannissement.

use crate::{
    backup::BackupInfo, history::{Sample, Session}, logs::LogChunk, monitor::Snapshot,
    settings::{Perm, RemoteSettings},
    Error, Result,
};
use axum::{
    extract::{Path, Query, Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use axum::extract::{ConnectInfo, Extension};
use serde::Serialize;
use std::{collections::HashSet, future::Future, net::{IpAddr, SocketAddr}, sync::{Arc, RwLock}, time::Duration};
use tokio::net::TcpListener;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control { Start, Stop, Restart }

/// Ce que l'API distante peut demander à l'application.
pub trait Backend: Clone + Send + Sync + 'static {
    fn snapshot(&self) -> impl Future<Output = Snapshot> + Send;
    fn control(&self, action: Control) -> impl Future<Output = Result<()>> + Send;
    fn history(&self, hours: i64) -> impl Future<Output = Vec<Sample>> + Send;
    fn sessions(&self, days: i64) -> impl Future<Output = Vec<Session>> + Send;
    fn logs(&self, offset: Option<u64>) -> impl Future<Output = Result<LogChunk>> + Send;
    fn backup_now(&self) -> impl Future<Output = Result<BackupInfo>> + Send;
    fn announce(&self, message: String) -> impl Future<Output = Result<()>> + Send;
    fn kick(&self, user_id: String) -> impl Future<Output = Result<()>> + Send;
}

type ApiErr = (StatusCode, String);
fn err(e: Error) -> ApiErr { (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()) }

/// Comparaison en temps constant (évite de révéler la clé par le temps de réponse).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Une clé et ce qu'elle autorise. Le propriétaire a tous les droits ; un invité seulement ceux accordés.
#[derive(Debug, Clone)]
pub struct Credential {
    pub token: String,
    pub name: String,
    pub admin: bool,
    pub perms: HashSet<Perm>,
    pub expires_at: Option<i64>,
}

impl Credential {
    fn can(&self, p: Perm) -> bool { self.admin || self.perms.contains(&p) }
}

/// Clés valides, modifiables à chaud : une invitation créée ou révoquée prend effet immédiatement,
/// sans redémarrer le serveur ni déconnecter les autres.
#[derive(Clone, Default)]
pub struct Credentials(Arc<RwLock<Vec<Credential>>>);

impl Credentials {
    pub fn new(v: Vec<Credential>) -> Self { Self(Arc::new(RwLock::new(v))) }
    pub fn set(&self, v: Vec<Credential>) { if let Ok(mut g) = self.0.write() { *g = v; } }

    /// Cherche la clé présentée. Toutes les entrées sont comparées (pas de sortie anticipée) ; une clé expirée est refusée.
    fn find(&self, presented: &str, now: i64) -> Option<Credential> {
        let g = self.0.read().ok()?;
        let mut found: Option<&Credential> = None;
        for c in g.iter() {
            if ct_eq(presented.as_bytes(), c.token.as_bytes()) { found = Some(c); }
        }
        found.filter(|c| c.expires_at.map_or(true, |e| now < e)).cloned()
    }
}

/// Clés issues des réglages : celle du propriétaire (si définie) + les invités.
pub fn credentials(cfg: &RemoteSettings) -> Vec<Credential> {
    let mut v = Vec::new();
    if cfg.token.len() >= 16 {
        v.push(Credential { token: cfg.token.clone(), name: "Propriétaire".into(), admin: true, perms: Perm::ALL.into_iter().collect(), expires_at: None });
    }
    for g in cfg.guests.iter().filter(|g| g.token.len() >= 16) {
        v.push(Credential { token: g.token.clone(), name: g.name.clone(), admin: false, perms: g.perms.iter().copied().collect(), expires_at: g.expires_at });
    }
    v
}

async fn auth(State(creds): State<Credentials>, mut req: Request, next: Next) -> Response {
    let found = req.headers().get(AUTHORIZATION).and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer ")).and_then(|t| creds.find(t, now_secs()));
    match found {
        Some(c) => { req.extensions_mut().insert(c); next.run(req).await }
        None => {
            tokio::time::sleep(Duration::from_millis(300)).await; // ralentit les essais répétés
            StatusCode::UNAUTHORIZED.into_response()
        }
    }
}

fn need(c: &Credential, p: Perm) -> std::result::Result<(), ApiErr> {
    if c.can(p) { Ok(()) } else { Err((StatusCode::FORBIDDEN, "droit insuffisant pour cette action".into())) }
}

#[derive(Serialize)]
struct Me { name: String, admin: bool, perms: Vec<Perm>, expires_at: Option<i64> }

async fn me(Extension(c): Extension<Credential>) -> Json<Me> {
    let mut perms: Vec<Perm> = Perm::ALL.into_iter().filter(|p| c.can(*p)).collect();
    perms.sort_by_key(|p| Perm::ALL.iter().position(|x| x == p));
    Json(Me { name: c.name, admin: c.admin, perms, expires_at: c.expires_at })
}

/// Adresses autorisées : boucle locale, réseaux privés, lien local et plage Tailscale (100.64.0.0/10, fd7a:115c:a1e0::/48).
/// Une adresse publique d'Internet est refusée MÊME avec la bonne clé : si un port est redirigé par erreur sur la box,
/// le serveur mobile ne répond quand même pas au monde entier.
pub fn is_allowed_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => v.is_loopback() || v.is_private() || v.is_link_local() || crate::net::is_cgnat(v),
        IpAddr::V6(v) => match v.to_ipv4_mapped() {
            Some(v4) => is_allowed_ip(IpAddr::V4(v4)),
            None => v.is_loopback() || (v.segments()[0] & 0xfe00) == 0xfc00 || (v.segments()[0] & 0xffc0) == 0xfe80,
        },
    }
}

async fn only_private(ConnectInfo(addr): ConnectInfo<SocketAddr>, req: Request, next: Next) -> Response {
    if is_allowed_ip(addr.ip()) { next.run(req).await } else { StatusCode::FORBIDDEN.into_response() }
}

async fn index() -> Html<&'static str> { Html(include_str!("mobile.html")) }

async fn snapshot<B: Backend>(State(b): State<B>, Extension(c): Extension<Credential>) -> std::result::Result<Json<Snapshot>, ApiErr> {
    need(&c, Perm::Status)?;
    let mut s = b.snapshot().await;
    if !c.can(Perm::Players) { s.players.clear(); } // les pseudos ne sont visibles qu'avec le droit « joueurs »
    Ok(Json(s))
}

async fn control<B: Backend>(State(b): State<B>, Extension(c): Extension<Credential>, Path(action): Path<String>) -> std::result::Result<StatusCode, ApiErr> {
    let (action, perm) = match action.as_str() {
        "start" => (Control::Start, Perm::Start), "stop" => (Control::Stop, Perm::Stop), "restart" => (Control::Restart, Perm::Restart),
        _ => return Err((StatusCode::BAD_REQUEST, "action inconnue".into())),
    };
    need(&c, perm)?;
    b.control(action).await.map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)] struct HistoryQ { hours: Option<i64> }
async fn history<B: Backend>(State(b): State<B>, Extension(c): Extension<Credential>, Query(q): Query<HistoryQ>) -> std::result::Result<Json<Vec<Sample>>, ApiErr> {
    need(&c, Perm::Charts)?;
    Ok(Json(b.history(q.hours.unwrap_or(6).clamp(1, 168)).await))
}

#[derive(Deserialize)] struct SessionsQ { days: Option<i64> }
async fn sessions<B: Backend>(State(b): State<B>, Extension(c): Extension<Credential>, Query(q): Query<SessionsQ>) -> std::result::Result<Json<Vec<Session>>, ApiErr> {
    need(&c, Perm::Players)?;
    Ok(Json(b.sessions(q.days.unwrap_or(7).clamp(1, 30)).await))
}

#[derive(Deserialize)] struct LogsQ { offset: Option<u64> }
async fn logs<B: Backend>(State(b): State<B>, Extension(c): Extension<Credential>, Query(q): Query<LogsQ>) -> std::result::Result<Json<LogChunk>, ApiErr> {
    need(&c, Perm::Logs)?;
    b.logs(q.offset).await.map(Json).map_err(err)
}

async fn backup_now<B: Backend>(State(b): State<B>, Extension(c): Extension<Credential>) -> std::result::Result<Json<BackupInfo>, ApiErr> {
    need(&c, Perm::Backup)?;
    b.backup_now().await.map(Json).map_err(err)
}

#[derive(Deserialize)] struct Msg { message: String }
async fn announce<B: Backend>(State(b): State<B>, Extension(c): Extension<Credential>, Json(m): Json<Msg>) -> std::result::Result<StatusCode, ApiErr> {
    need(&c, Perm::Announce)?;
    if m.message.trim().is_empty() || m.message.len() > 500 { return Err((StatusCode::BAD_REQUEST, "message invalide".into())); }
    b.announce(m.message).await.map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)] struct Kick { user_id: String }
async fn kick<B: Backend>(State(b): State<B>, Extension(c): Extension<Credential>, Json(k): Json<Kick>) -> std::result::Result<StatusCode, ApiErr> {
    need(&c, Perm::Kick)?;
    b.kick(k.user_id).await.map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

/// La page mobile est publique (elle ne contient aucune donnée) ; toute l'API exige le jeton.
pub fn router<B: Backend>(backend: B, creds: Credentials) -> Router {
    let api = Router::new()
        .route("/me", get(me))
        .route("/snapshot", get(snapshot::<B>))
        .route("/control/:action", post(control::<B>))
        .route("/history", get(history::<B>))
        .route("/sessions", get(sessions::<B>))
        .route("/logs", get(logs::<B>))
        .route("/backup", post(backup_now::<B>))
        .route("/announce", post(announce::<B>))
        .route("/kick", post(kick::<B>))
        .layer(middleware::from_fn_with_state(creds, auth))
        .with_state(backend);
    Router::new().route("/", get(index)).nest("/api", api).layer(middleware::from_fn(only_private))
}

/// Écoute sur toutes les interfaces (accès depuis le téléphone). Échoue si le port est pris.
pub async fn bind(port: u16) -> std::io::Result<TcpListener> {
    TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).await
}

pub async fn run<B: Backend>(listener: TcpListener, backend: B, creds: Credentials) -> std::io::Result<()> {
    axum::serve(listener, router(backend, creds).into_make_service_with_connect_info::<SocketAddr>()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Clone, Default)]
    struct Fake { calls: Arc<Mutex<Vec<String>>> }

    impl Backend for Fake {
        async fn snapshot(&self) -> Snapshot { Snapshot { running: true, cpu_percent: 12.0, ..Default::default() } }
        async fn control(&self, a: Control) -> Result<()> { self.calls.lock().unwrap().push(format!("{a:?}")); Ok(()) }
        async fn history(&self, _: i64) -> Vec<Sample> { vec![] }
        async fn sessions(&self, _: i64) -> Vec<Session> { vec![] }
        async fn logs(&self, _: Option<u64>) -> Result<LogChunk> { Ok(LogChunk { lines: vec!["hello".into()], offset: 6 }) }
        async fn backup_now(&self) -> Result<BackupInfo> { Err(Error::Other("pas de serveur".into())) }
        async fn announce(&self, m: String) -> Result<()> { self.calls.lock().unwrap().push(format!("announce:{m}")); Ok(()) }
        async fn kick(&self, u: String) -> Result<()> { self.calls.lock().unwrap().push(format!("kick:{u}")); Ok(()) }
    }

    fn cred(token: &str, name: &str, admin: bool, perms: &[Perm], expires_at: Option<i64>) -> Credential {
        Credential { token: token.into(), name: name.into(), admin, perms: perms.iter().copied().collect(), expires_at }
    }

    async fn start_with(fake: Fake, creds: Credentials) -> String {
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        tokio::spawn(run(l, fake, creds));
        base
    }

    async fn start(fake: Fake) -> String {
        start_with(fake, Credentials::new(vec![cred("secret-token-1234567", "Propriétaire", true, &Perm::ALL, None)])).await
    }

    #[tokio::test]
    async fn auth_is_required_for_api_but_not_for_page() {
        let base = start(Fake::default()).await;
        let c = reqwest::Client::new();
        assert_eq!(c.get(format!("{base}/")).send().await.unwrap().status(), 200);
        assert_eq!(c.get(format!("{base}/api/snapshot")).send().await.unwrap().status(), 401);
        assert_eq!(c.get(format!("{base}/api/snapshot")).bearer_auth("wrong").send().await.unwrap().status(), 401);
        assert_eq!(c.post(format!("{base}/api/control/restart")).send().await.unwrap().status(), 401);
        let ok = c.get(format!("{base}/api/snapshot")).bearer_auth("secret-token-1234567").send().await.unwrap();
        assert_eq!(ok.status(), 200);
        assert_eq!(ok.json::<serde_json::Value>().await.unwrap()["running"], true);
    }

    #[tokio::test]
    async fn control_and_actions_reach_backend() {
        let fake = Fake::default();
        let base = start(fake.clone()).await;
        let c = reqwest::Client::new();
        let t = "secret-token-1234567";
        assert_eq!(c.post(format!("{base}/api/control/restart")).bearer_auth(t).send().await.unwrap().status(), 204);
        assert_eq!(c.post(format!("{base}/api/control/reboot")).bearer_auth(t).send().await.unwrap().status(), 400);
        assert_eq!(c.post(format!("{base}/api/announce")).bearer_auth(t).json(&serde_json::json!({"message":"salut"})).send().await.unwrap().status(), 204);
        assert_eq!(c.post(format!("{base}/api/announce")).bearer_auth(t).json(&serde_json::json!({"message":"  "})).send().await.unwrap().status(), 400);
        assert_eq!(c.post(format!("{base}/api/kick")).bearer_auth(t).json(&serde_json::json!({"user_id":"steam_1"})).send().await.unwrap().status(), 204);
        assert_eq!(c.post(format!("{base}/api/backup")).bearer_auth(t).send().await.unwrap().status(), 500);
        assert_eq!(c.get(format!("{base}/api/logs?offset=0")).bearer_auth(t).send().await.unwrap().status(), 200);
        // Routes sensibles : inexistantes côté distant.
        for p in ["settings", "config", "restore", "update", "ban"] {
            assert_eq!(c.get(format!("{base}/api/{p}")).bearer_auth(t).send().await.unwrap().status(), 404, "{p}");
        }
        assert_eq!(*fake.calls.lock().unwrap(), ["Restart", "announce:salut", "kick:steam_1"]);
    }

    #[tokio::test]
    async fn guest_permissions_expiry_and_revocation() {
        let fake = Fake::default();
        let creds = Credentials::new(vec![
            cred("owner-token-1234567890", "Propriétaire", true, &Perm::ALL, None),
            cred("viewer-token-123456789", "Alice", false, &[Perm::Status], None),
            cred("mod-token-12345678901", "Bob", false, &[Perm::Status, Perm::Restart, Perm::Players], None),
            cred("old-token-123456789012", "Expiré", false, &[Perm::Status], Some(1)),
        ]);
        let base = start_with(fake.clone(), creds.clone()).await;
        let c = reqwest::Client::new();
        let get = |p: &str, t: &str| c.get(format!("{base}{p}")).bearer_auth(t.to_string()).send();
        let post = |p: &str, t: &str| c.post(format!("{base}{p}")).bearer_auth(t.to_string()).send();

        // /me décrit les droits réels.
        let me: serde_json::Value = get("/api/me", "viewer-token-123456789").await.unwrap().json().await.unwrap();
        assert_eq!((me["name"].as_str(), me["admin"].as_bool(), me["perms"].clone()), (Some("Alice"), Some(false), serde_json::json!(["status"])));
        // Spectateur : voit l'état, rien d'autre.
        assert_eq!(get("/api/snapshot", "viewer-token-123456789").await.unwrap().status(), 200);
        assert_eq!(get("/api/logs", "viewer-token-123456789").await.unwrap().status(), 403);
        assert_eq!(get("/api/history", "viewer-token-123456789").await.unwrap().status(), 403);
        assert_eq!(get("/api/sessions", "viewer-token-123456789").await.unwrap().status(), 403);
        assert_eq!(post("/api/control/restart", "viewer-token-123456789").await.unwrap().status(), 403);
        assert_eq!(post("/api/control/stop", "viewer-token-123456789").await.unwrap().status(), 403);
        assert_eq!(post("/api/backup", "viewer-token-123456789").await.unwrap().status(), 403);
        assert_eq!(c.post(format!("{base}/api/kick")).bearer_auth("viewer-token-123456789").json(&serde_json::json!({"user_id":"x"})).send().await.unwrap().status(), 403);
        // Modérateur : redémarrer oui, arrêter non.
        assert_eq!(post("/api/control/restart", "mod-token-12345678901").await.unwrap().status(), 204);
        assert_eq!(post("/api/control/stop", "mod-token-12345678901").await.unwrap().status(), 403);
        assert_eq!(get("/api/sessions", "mod-token-12345678901").await.unwrap().status(), 200);
        // Clé expirée ou inconnue : 401.
        assert_eq!(get("/api/snapshot", "old-token-123456789012").await.unwrap().status(), 401);
        assert_eq!(get("/api/snapshot", "nimporte-quoi").await.unwrap().status(), 401);
        // Révocation immédiate, sans redémarrer le serveur.
        creds.set(vec![cred("owner-token-1234567890", "Propriétaire", true, &Perm::ALL, None)]);
        assert_eq!(get("/api/snapshot", "viewer-token-123456789").await.unwrap().status(), 401);
        assert_eq!(get("/api/snapshot", "owner-token-1234567890").await.unwrap().status(), 200);
        assert_eq!(*fake.calls.lock().unwrap(), ["Restart"]);
    }

    #[test]
    fn credentials_from_settings() {
        let mut cfg = RemoteSettings { token: "t".repeat(32), ..Default::default() };
        cfg.guests.push(crate::settings::Guest { id: "g1".into(), name: "Ami".into(), token: "g".repeat(32), perms: vec![Perm::Status], expires_at: None, created_at: 0 });
        cfg.guests.push(crate::settings::Guest { id: "g2".into(), name: "Court".into(), token: "x".into(), perms: vec![Perm::Status], expires_at: None, created_at: 0 }); // clé trop courte : ignorée
        let v = credentials(&cfg);
        assert_eq!(v.len(), 2);
        assert!(v[0].admin && v[0].perms.len() == Perm::ALL.len());
        assert!(!v[1].admin && v[1].perms.len() == 1);
        assert!(credentials(&RemoteSettings::default()).is_empty());
    }

    #[test]
    fn only_private_and_tailscale_addresses_are_allowed() {
        for ok in ["127.0.0.1", "192.168.1.20", "10.0.0.5", "172.16.3.4", "169.254.1.1", "100.100.1.2", "::1", "fd7a:115c:a1e0::1", "fe80::1", "::ffff:192.168.0.2"] {
            assert!(is_allowed_ip(ok.parse().unwrap()), "{ok}");
        }
        for ko in ["8.8.8.8", "203.0.113.7", "100.63.0.1", "100.128.0.1", "172.32.0.1", "2001:db8::1", "::ffff:8.8.8.8"] {
            assert!(!is_allowed_ip(ko.parse().unwrap()), "{ko}");
        }
    }

    #[test]
    fn token_generation_and_ct_eq() {
        let a = RemoteSettings::generate_token().unwrap();
        assert_eq!(a.len(), 32);
        assert_ne!(a, RemoteSettings::generate_token().unwrap());
        assert!(ct_eq(b"abc", b"abc") && !ct_eq(b"abc", b"abd") && !ct_eq(b"abc", b"ab"));
    }
    use crate::settings::RemoteSettings;
}
