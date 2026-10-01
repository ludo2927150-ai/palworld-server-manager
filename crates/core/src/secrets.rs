//! Secrets hors de `settings.json` : mot de passe admin, webhooks, jetons d'accès distant.
//!
//! Les valeurs sensibles sont confiées à un `SecretStore` (le gestionnaire d'identifiants de Windows
//! côté application) et remplacées dans le fichier par le marqueur `@secret`. Sans magasin disponible,
//! ou si une écriture échoue, la valeur reste en clair : on ne perd jamais un secret.

use crate::settings::AppSettings;
use std::{collections::HashMap, sync::{Mutex, OnceLock}};

pub const MARKER: &str = "@secret";

pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, value: &str) -> bool;
    fn delete(&self, key: &str);
}

/// Magasin en mémoire (tests).
#[derive(Default)]
pub struct MemoryStore(Mutex<HashMap<String, String>>);
impl MemoryStore {
    pub fn len(&self) -> usize { self.0.lock().unwrap().len() }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
}
impl SecretStore for MemoryStore {
    fn get(&self, key: &str) -> Option<String> { self.0.lock().unwrap().get(key).cloned() }
    fn set(&self, key: &str, value: &str) -> bool { self.0.lock().unwrap().insert(key.into(), value.into()); true }
    fn delete(&self, key: &str) { self.0.lock().unwrap().remove(key); }
}

static STORE: OnceLock<Box<dyn SecretStore>> = OnceLock::new();

/// Installe le magasin global (une seule fois, au démarrage de l'application).
pub fn install(store: Box<dyn SecretStore>) { let _ = STORE.set(store); }
pub fn global() -> Option<&'static dyn SecretStore> { STORE.get().map(|b| b.as_ref()) }

fn protect_one(store: &dyn SecretStore, key: &str, value: &mut String) {
    if value.is_empty() || value == MARKER { return; }
    if store.set(key, value) && store.get(key).as_deref() == Some(value.as_str()) { *value = MARKER.into(); }
}
fn reveal_one(store: &dyn SecretStore, key: &str, value: &mut String) {
    if value == MARKER { *value = store.get(key).unwrap_or_default(); }
}

/// Copie à écrire sur disque : secrets déplacés dans le magasin, marqueurs à la place.
pub fn protect(store: &dyn SecretStore, s: &AppSettings) -> AppSettings {
    let mut o = s.clone();
    protect_one(store, "rest.admin_password", &mut o.rest.admin_password);
    protect_one(store, "remote.token", &mut o.remote.token);
    for g in &mut o.remote.guests { protect_one(store, &format!("guest.{}", g.id), &mut g.token); }
    for (k, f) in [("alerts.discord_webhook", &mut o.alerts.discord_webhook), ("alerts.ntfy_url", &mut o.alerts.ntfy_url)] {
        if let Some(v) = f { protect_one(store, k, v); }
    }
    o
}

/// Remplace les marqueurs lus sur disque par les vraies valeurs.
pub fn reveal(store: &dyn SecretStore, s: &mut AppSettings) {
    reveal_one(store, "rest.admin_password", &mut s.rest.admin_password);
    reveal_one(store, "remote.token", &mut s.remote.token);
    for g in &mut s.remote.guests { reveal_one(store, &format!("guest.{}", g.id), &mut g.token); }
    for (k, f) in [("alerts.discord_webhook", &mut s.alerts.discord_webhook), ("alerts.ntfy_url", &mut s.alerts.ntfy_url)] {
        if let Some(v) = f { reveal_one(store, k, v); }
    }
}

/// Supprime les secrets d'invités qui n'existent plus.
pub fn prune_guests(store: &dyn SecretStore, before: &AppSettings, after: &AppSettings) {
    for g in &before.remote.guests {
        if !after.remote.guests.iter().any(|x| x.id == g.id) { store.delete(&format!("guest.{}", g.id)); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Guest;

    struct Broken;
    impl SecretStore for Broken {
        fn get(&self, _: &str) -> Option<String> { None }
        fn set(&self, _: &str, _: &str) -> bool { false }
        fn delete(&self, _: &str) {}
    }

    fn sample() -> AppSettings {
        let mut s = AppSettings::default();
        s.rest.admin_password = "pw".into();
        s.remote.token = "tok".into();
        s.alerts.discord_webhook = Some("https://discord/hook".into());
        s.remote.guests.push(Guest { id: "g1".into(), name: "Bob".into(), token: "gt".into(), perms: vec![], expires_at: None, created_at: 0 });
        s
    }

    #[test]
    fn roundtrip_moves_secrets_out_of_the_file() {
        let store = MemoryStore::default();
        let s = sample();
        let disk = protect(&store, &s);
        let json = serde_json::to_string(&disk).unwrap();
        for secret in ["\"pw\"", "\"tok\"", "discord/hook", "\"gt\""] { assert!(!json.contains(secret), "{secret} fuite"); }
        assert_eq!(store.len(), 4);
        let mut back = disk.clone();
        reveal(&store, &mut back);
        assert_eq!(back.rest.admin_password, "pw");
        assert_eq!(back.remote.guests[0].token, "gt");
        assert_eq!(back.alerts.discord_webhook.as_deref(), Some("https://discord/hook"));
    }

    #[test]
    fn broken_store_keeps_plaintext() {
        let disk = protect(&Broken, &sample());
        assert_eq!(disk.rest.admin_password, "pw");
        assert_eq!(disk.remote.token, "tok");
    }

    #[test]
    fn plaintext_files_are_migrated_and_empty_values_untouched() {
        let store = MemoryStore::default();
        let mut s = AppSettings::default();
        let once = protect(&store, &s);
        assert!(store.is_empty() && once.rest.admin_password.is_empty());
        s.rest.admin_password = "x".into();
        let p1 = protect(&store, &s);
        let p2 = protect(&store, &p1); // idempotent : le marqueur n'est pas re-stocké
        assert_eq!(p2.rest.admin_password, MARKER);
        assert_eq!(store.get("rest.admin_password").as_deref(), Some("x"));
    }

    #[test]
    fn removed_guests_are_pruned() {
        let store = MemoryStore::default();
        let a = sample();
        protect(&store, &a);
        let mut b = a.clone();
        b.remote.guests.clear();
        prune_guests(&store, &a, &b);
        assert!(store.get("guest.g1").is_none());
    }
}
