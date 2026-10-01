# Palworld Server Manager

Application desktop Windows (Tauri 2 + React + Tailwind) pour piloter un serveur dédié Palworld.

| Fonction | État |
|---|---|
| Démarrer / arrêter (propre via API REST) / redémarrer `PalServer.exe` | scaffold fonctionnel |
| Éditeur graphique de `PalWorldSettings.ini` (aller-retour fidèle) | scaffold fonctionnel |
| Monitoring CPU / RAM / joueurs / FPS | scaffold fonctionnel |
| Backups ZIP automatiques + rotation + restauration | scaffold fonctionnel (testé) |
| Horaires programmés (démarrer / arrêter / redémarrer, par jour de semaine) + annonces en jeu, redémarrage sur seuil RAM | scaffold fonctionnel (planificateur testé) |
| Mise à jour SteamCMD, adoption d'un serveur déjà lancé, ban/unban | scaffold fonctionnel |
| Historique 7 j (graphiques CPU/RAM/joueurs/FPS), sessions de jeu, journal du serveur en direct | scaffold fonctionnel (testé) |
| Éditeur de config complet (~90 clés, préréglages, diff, import/export), diagnostic + correction API REST | scaffold fonctionnel |
| Sauvegarde systématique à chaque arrêt/redémarrage (même arrêt externe), intervalle réglable, AutoSaveSpan du jeu | scaffold fonctionnel (testé) |
| Copie des sauvegardes vers un second emplacement, résumé quotidien Discord/ntfy | scaffold fonctionnel (testé) |
| Icône de zone de notification, lancement avec Windows, fermer = réduire | scaffold (à valider sur Windows) |
| Alertes Discord + push mobile (ntfy) : crash, mémoire, connexions | scaffold fonctionnel (moteur testé) |

> ⚠️ Dans un navigateur (`npm run dev`) l'interface affiche des **données fictives** (bandeau « MODE DÉMO »). Seule l'application Tauri pilote un vrai serveur.

## Démarrage rapide

Prérequis : Node 20+, Rust stable, [prérequis Tauri pour Windows](https://tauri.app/start/prerequisites/) (WebView2, MSVC).

```bash
npm install
# (optionnel) npm run tauri icon logo.png  -> remplace les icônes fournies dans src-tauri/icons/
npm run tauri dev                      # app complète
npm run dev                            # UI seule dans un navigateur, avec données factices
cargo test -p palmanager-core          # tests de la logique métier
npm run tauri build                    # installeur NSIS/MSI
```

### Suivre les modifications en direct

```powershell
.\scripts\dev-live.ps1        # lance l'app + tire automatiquement les nouveaux commits (toutes les 10 s)
```

**Raccourci** : double-cliquez sur `Lancer-Palworld-Manager.bat` (racine du dépôt). Pour un raccourci sur le Bureau : `powershell -ExecutionPolicy Bypass -File .\scripts\creer-raccourci.ps1`.

Les changements React/CSS s'affichent à chaud dans la fenêtre ; les changements Rust déclenchent une recompilation et un relancement automatiques. Si PowerShell refuse le script : `powershell -ExecutionPolicy Bypass -File .\scripts\dev-live.ps1`.

Côté serveur Palworld, activer l'API REST dans `PalWorldSettings.ini` :
`RESTAPIEnabled=True`, `RESTAPIPort=8212`, et définir `AdminPassword` (à recopier dans l'onglet « Application »).
Ne pas exposer ce port sur Internet.

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Feuille de route](docs/ROADMAP.md)
- [Contribuer](CONTRIBUTING.md)
