//! Client de l'API REST officielle de Palworld (`RESTAPIEnabled=True` dans PalWorldSettings.ini).
//! Auth HTTP Basic `admin:<AdminPassword>`, base `http://host:8212/v1/api`.

use crate::{settings::RestSettings, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Player {
    pub name: String,
    #[serde(rename = "accountName", default)]
    pub account_name: String,
    #[serde(rename = "playerId", default)]
    pub player_id: String,
    #[serde(rename = "userId", default)]
    pub user_id: String,
    #[serde(default)]
    pub level: u32,
    #[serde(default)]
    pub ping: f32,
    /// Position dans le monde (coordonnées du jeu) et nombre de constructions, si l'API les fournit.
    #[serde(default)]
    pub location_x: f64,
    #[serde(default)]
    pub location_y: f64,
    #[serde(default)]
    pub building_count: u32,
}

#[derive(Debug, Deserialize)]
struct PlayersResponse {
    #[serde(default)]
    players: Vec<Player>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Metrics {
    #[serde(default)] pub currentplayernum: u32,
    #[serde(default)] pub maxplayernum: u32,
    #[serde(default)] pub serverfps: u32,
    #[serde(default)] pub days: u32,
    #[serde(default)] pub uptime: u64,
}

#[derive(Clone)]
pub struct RestClient {
    http: reqwest::Client,
    base: String,
    password: String,
}

impl RestClient {
    pub fn new(cfg: &RestSettings) -> Result<Self> {
        Ok(Self {
            http: reqwest::Client::builder().timeout(Duration::from_secs(5)).build()?,
            base: format!("http://{}:{}/v1/api", cfg.host, cfg.port),
            password: cfg.admin_password.clone(),
        })
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        Ok(self.http.get(format!("{}/{path}", self.base)).basic_auth("admin", Some(&self.password))
            .send().await?.error_for_status()?.json().await?)
    }
    async fn post(&self, path: &str, body: serde_json::Value) -> Result<()> {
        self.http.post(format!("{}/{path}", self.base)).basic_auth("admin", Some(&self.password))
            .json(&body).send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn players(&self) -> Result<Vec<Player>> { Ok(self.get::<PlayersResponse>("players").await?.players) }
    pub async fn metrics(&self) -> Result<Metrics> { self.get("metrics").await }
    pub async fn announce(&self, message: &str) -> Result<()> { self.post("announce", serde_json::json!({ "message": message })).await }
    pub async fn save(&self) -> Result<()> { self.post("save", serde_json::json!({})).await }
    pub async fn shutdown(&self, wait_secs: u32, message: &str) -> Result<()> {
        self.post("shutdown", serde_json::json!({ "waittime": wait_secs, "message": message })).await
    }
    pub async fn kick(&self, user_id: &str, message: &str) -> Result<()> {
        self.post("kick", serde_json::json!({ "userid": user_id, "message": message })).await
    }
    pub async fn unban(&self, user_id: &str) -> Result<()> { self.post("unban", serde_json::json!({ "userid": user_id })).await }
    pub async fn ban(&self, user_id: &str, message: &str) -> Result<()> {
        self.post("ban", serde_json::json!({ "userid": user_id, "message": message })).await
    }
}
