//! Logique métier du Palworld Server Manager, indépendante de Tauri.
//! La couche `src-tauri` n'expose que de fines commandes IPC au-dessus de ces modules.

pub mod alerts;
pub mod backup;
pub mod error;
pub mod ini;
pub mod monitor;
pub mod rest;
pub mod schedule;
pub mod server;
pub mod settings;
pub mod steamcmd;

pub use error::{Error, Result};
