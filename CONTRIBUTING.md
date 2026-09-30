# Contribuer

- Logique métier → `crates/core` avec tests unitaires ; `src-tauri` reste une couche d'adaptation.
- Toute struct sérialisée modifiée côté Rust doit être répercutée dans `src/lib/types.ts`.
- Avant une PR : `cargo test -p palmanager-core`, `cargo clippy --workspace`, `npm run build`.
