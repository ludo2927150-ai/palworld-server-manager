//! Magasin de secrets : Gestionnaire d'identifiants Windows (hors Windows : aucun, les valeurs restent dans settings.json).

#[cfg(windows)]
struct Keyring;

#[cfg(windows)]
impl palmanager_core::secrets::SecretStore for Keyring {
    fn get(&self, key: &str) -> Option<String> {
        keyring::Entry::new("PalworldServerManager", key).ok()?.get_password().ok()
    }
    fn set(&self, key: &str, value: &str) -> bool {
        keyring::Entry::new("PalworldServerManager", key).and_then(|e| e.set_password(value)).is_ok()
    }
    fn delete(&self, key: &str) {
        if let Ok(e) = keyring::Entry::new("PalworldServerManager", key) { let _ = e.delete_credential(); }
    }
}

/// À appeler avant le premier chargement des réglages.
pub fn install() {
    #[cfg(windows)]
    palmanager_core::secrets::install(Box::new(Keyring));
}
