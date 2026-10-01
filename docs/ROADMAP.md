# Feuille de route

**v0.1 (ce scaffold)** — structure, contrôle, config, monitoring, backups, alertes de base.

**Limites connues à traiter en priorité**
- Compilation de `src-tauri` non validée dans l'environnement de génération (pas de WebView Linux) : première tâche = `npm run tauri dev` sur Windows et corriger les éventuelles erreurs.
- Le schéma de `PalWorldSettings.ini` couvre ~18 clés ; les autres passent par l'onglet « Avancé ».

**v0.2** — graphiques historiques CPU/RAM, logs du serveur, planification de redémarrages avec annonces in-game, mise à jour via SteamCMD.

**v0.3** — Credential Manager, icône de zone de notification (tray), démarrage avec Windows, restauration guidée avec arrêt/relance automatiques.

**Lot 1 (fait)** — redémarrages planifiés + annonces, seuil RAM, SteamCMD, adoption par nom de processus, ban/unban.
**Lot 2 (fait)** — historique/graphiques, sessions, journal en direct, schéma de config ~90 clés + préréglages/diff/import-export, diagnostic de premier lancement.
**Lot 3 (fait)** — backups vers un 2ᵉ emplacement, résumé quotidien, icône de zone de notification, lancement avec Windows.
**Fait depuis** — gestion des joueurs (historique, bannis, liste blanche), assistant de premier lancement, secrets dans le Gestionnaire d'identifiants Windows, vérification/installation des mises à jour via les Releases GitHub, sélecteur FR/EN (traduction partielle : navigation, bannières, assistant, page Joueurs).
**Reste** — finir la traduction anglaise des autres pages (`src/lib/i18n.ts`, le français sert de clé), signature de code de l'installeur (nécessite un certificat payant ; sans lui SmartScreen avertit), accès invité hors Tailscale (à n'ouvrir qu'avec accord explicite).
