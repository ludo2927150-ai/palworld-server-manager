//! Accès à distance : API JSON + page mobile, protégées par un jeton. Le routeur est générique sur
//! un `Backend` (implémenté par la couche Tauri) : il est donc testable sans Tauri.
//!
//! Volontairement exposé : état, historique, journal, démarrer/arrêter/redémarrer, sauvegarde, annonce,
//! expulsion. Volontairement NON exposé : paramètres de l'application (secrets), écriture de la
//! configuration du monde, restauration de sauvegarde, mise à jour SteamCMD, bannissement.

use crate::{
    backup::BackupInfo, history::{Sample, Session}, logs::LogChunk, monitor::Snapshot, Error, Result,
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
use axum::extract::ConnectInfo;
use std::{future::Future, net::{IpAddr, SocketAddr}, sync::Arc, time::Duration};
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

/// Comparaison en temps constant (évite de révéler le jeton par le temps de réponse).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

async fn auth(State(token): State<Arc<String>>, req: Request, next: Next) -> Response {
    let ok = req.headers().get(AUTHORIZATION).and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer ")).is_some_and(|t| ct_eq(t.as_bytes(), token.as_bytes()));
    if ok { return next.run(req).await; }
    tokio::time::sleep(Duration::from_millis(300)).await; // ralentit les essais répétés
    StatusCode::UNAUTHORIZED.into_response()
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

async fn snapshot<B: Backend>(State(b): State<B>) -> Json<Snapshot> { Json(b.snapshot().await) }

async fn control<B: Backend>(State(b): State<B>, Path(action): Path<String>) -> std::result::Result<StatusCode, ApiErr> {
    let action = match action.as_str() {
        "start" => Control::Start, "stop" => Control::Stop, "restart" => Control::Restart,
        _ => return Err((StatusCode::BAD_REQUEST, "action inconnue".into())),
    };
    b.control(action).await.map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)] struct HistoryQ { hours: Option<i64> }
async fn history<B: Backend>(State(b): State<B>, Query(q): Query<HistoryQ>) -> Json<Vec<Sample>> {
    Json(b.history(q.hours.unwrap_or(6).clamp(1, 168)).await)
}

#[derive(Deserialize)] struct SessionsQ { days: Option<i64> }
async fn sessions<B: Backend>(State(b): State<B>, Query(q): Query<SessionsQ>) -> Json<Vec<Session>> {
    Json(b.sessions(q.days.unwrap_or(7).clamp(1, 30)).await)
}

#[derive(Deserialize)] struct LogsQ { offset: Option<u64> }
async fn logs<B: Backend>(State(b): State<B>, Query(q): Query<LogsQ>) -> std::result::Result<Json<LogChunk>, ApiErr> {
    b.logs(q.offset).await.map(Json).map_err(err)
}

async fn backup_now<B: Backend>(State(b): State<B>) -> std::result::Result<Json<BackupInfo>, ApiErr> {
    b.backup_now().await.map(Json).map_err(err)
}

#[derive(Deserialize)] struct Msg { message: String }
async fn announce<B: Backend>(State(b): State<B>, Json(m): Json<Msg>) -> std::result::Result<StatusCode, ApiErr> {
    if m.message.trim().is_empty() || m.message.len() > 500 { return Err((StatusCode::BAD_REQUEST, "message invalide".into())); }
    b.announce(m.message).await.map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)] struct Kick { user_id: String }
async fn kick<B: Backend>(State(b): State<B>, Json(k): Json<Kick>) -> std::result::Result<StatusCode, ApiErr> {
    b.kick(k.user_id).await.map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

/// La page mobile est publique (elle ne contient aucune donnée) ; toute l'API exige le jeton.
pub fn router<B: Backend>(backend: B, token: String) -> Router {
    let api = Router::new()
        .route("/snapshot", get(snapshot::<B>))
        .route("/control/:action", post(control::<B>))
        .route("/history", get(history::<B>))
        .route("/sessions", get(sessions::<B>))
        .route("/logs", get(logs::<B>))
        .route("/backup", post(backup_now::<B>))
        .route("/announce", post(announce::<B>))
        .route("/kick", post(kick::<B>))
        .layer(middleware::from_fn_with_state(Arc::new(token), auth))
        .with_state(backend);
    Router::new().route("/", get(index)).nest("/api", api).layer(middleware::from_fn(only_private))
}

/// Écoute sur toutes les interfaces (accès depuis le téléphone). Échoue si le port est pris.
pub async fn bind(port: u16) -> std::io::Result<TcpListener> {
    TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).await
}

pub async fn run<B: Backend>(listener: TcpListener, backend: B, token: String) -> std::io::Result<()> {
    axum::serve(listener, router(backend, token).into_make_service_with_connect_info::<SocketAddr>()).await
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

    async fn start(fake: Fake) -> String {
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        tokio::spawn(run(l, fake, "secret-token-1234567".into()));
        base
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
