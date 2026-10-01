//! Logique métier du Palworld Server Manager, indépendante de Tauri.
//! La couche `src-tauri` n'expose que de fines commandes IPC au-dessus de ces modules.

pub mod alerts;
pub mod announce;
pub mod audit;
pub mod autostart;
pub mod backup;
pub mod discord;
pub mod error;
pub mod health;
pub mod history;
pub mod ini;
pub mod install;
pub mod loganalysis;
pub mod lock;
pub mod logs;
pub mod mods;
pub mod monitor;
pub mod net;
pub mod perf;
pub mod players;
pub mod playersaves;
pub mod profiles;
pub mod remote;
pub mod rest;
pub mod restoretest;
pub mod schedule;
pub mod season;
pub mod secrets;
pub mod server;
pub mod settings;
pub mod summary;
pub mod update;
pub mod upnp;
pub mod watchdog;
pub mod setup;
pub mod steamcmd;

pub use error::{Error, Result};
