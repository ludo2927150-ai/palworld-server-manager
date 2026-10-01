//! Verrou d'affichage par code (voir `LockSettings`). Le code n'est jamais stocké : seulement un condensat salé
//! (SHA-256 itéré 100 000 fois).

use crate::{settings::LockSettings, Error, Result};
use sha2::{Digest, Sha256};

const ROUNDS: u32 = 100_000;

fn derive(salt: &str, pin: &str) -> String {
    let mut h = Sha256::digest(format!("{salt}:{pin}").as_bytes());
    for _ in 0..ROUNDS { h = Sha256::digest(h); }
    h.iter().map(|b| format!("{b:02x}")).collect()
}

/// Définit (ou remplace) le code. 4 à 32 caractères.
pub fn set_pin(lock: &mut LockSettings, pin: &str) -> Result<()> {
    let n = pin.chars().count();
    if !(4..=32).contains(&n) { return Err(Error::Other("le code doit faire entre 4 et 32 caractères".into())); }
    let mut s = [0u8; 16];
    getrandom::getrandom(&mut s).map_err(|e| Error::Other(e.to_string()))?;
    lock.salt = s.iter().map(|b| format!("{b:02x}")).collect();
    lock.hash = derive(&lock.salt, pin);
    lock.enabled = true;
    Ok(())
}

/// Comparaison en temps constant.
pub fn verify(lock: &LockSettings, pin: &str) -> bool {
    if !lock.enabled || lock.hash.len() != 64 { return true; } // pas de verrou : rien à vérifier
    let (a, b) = (derive(&lock.salt, pin), &lock.hash);
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub fn clear(lock: &mut LockSettings) { *lock = LockSettings { auto_lock_minutes: lock.auto_lock_minutes, ..Default::default() }; }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_verify_clear() {
        let mut l = LockSettings::default();
        assert!(verify(&l, "n'importe quoi"), "sans verrou tout passe");
        assert!(set_pin(&mut l, "123").is_err() && set_pin(&mut l, &"x".repeat(40)).is_err());
        set_pin(&mut l, "mon-code").unwrap();
        assert!(l.enabled && !l.hash.contains("mon-code") && !l.salt.is_empty());
        assert!(verify(&l, "mon-code") && !verify(&l, "mon-codE") && !verify(&l, ""));
        let first = l.hash.clone();
        set_pin(&mut l, "mon-code").unwrap();
        assert_ne!(first, l.hash, "sel différent à chaque définition");
        clear(&mut l);
        assert!(!l.enabled && verify(&l, "x"));
    }
}
