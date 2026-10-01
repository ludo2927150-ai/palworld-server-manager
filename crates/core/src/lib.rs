//! Logique métier du Palworld Server Manager, indépendante de Tauri.
//! La couche `src-tauri` n'expose que de fines commandes IPC au-dessus de ces modules.

pub mod alerts;
pub mod autostart;
pub mod backup;
pub mod error;
pub mod history;
pub mod ini;
pub mod logs;
pub mod monitor;
pub mod rest;
pub mod schedule;
pub mod server;
pub mod settings;
pub mod summary;
pub mod setup;
pub mod steamcmd;

pub use error::{Error, Result};
