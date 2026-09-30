# Feuille de route

**v0.1 (ce scaffold)** — structure, contrôle, config, monitoring, backups, alertes de base.

**Limites connues à traiter en priorité**
- Compilation de `src-tauri` non validée dans l'environnement de génération (pas de WebView Linux) : première tâche = `npm run tauri dev` sur Windows et corriger les éventuelles erreurs.
- Icônes à générer (`npm run tauri icon`).
- Le schéma de `PalWorldSettings.ini` couvre ~18 clés ; les autres passent par l'onglet « Avancé ».

**v0.2** — graphiques historiques CPU/RAM, logs du serveur, planification de redémarrages avec annonces in-game, mise à jour via SteamCMD.

**v0.3** — Credential Manager, icône de zone de notification (tray), démarrage avec Windows, restauration guidée avec arrêt/relance automatiques.

**Lot 1 (fait)** — redémarrages planifiés + annonces, seuil RAM, SteamCMD, adoption par nom de processus, ban/unban.
**Lot 2 (fait)** — historique/graphiques, sessions, journal en direct, schéma de config ~90 clés + préréglages/diff/import-export, diagnostic de premier lancement.
**Lot 3 (idée)** — Credential Manager, icône de zone de notification, démarrage avec Windows, backups vers un 2ᵉ emplacement, résumé quotidien Discord, i18n FR/EN.
