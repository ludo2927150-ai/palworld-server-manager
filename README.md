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
| Onglet Performance : limite de RAM (alerte/redémarrage), priorité CPU, choix des cœurs, options de threads | scaffold (priorité/cœurs à valider sur Windows) |
| Accès depuis le téléphone (page web + clé secrète, Tailscale pour la 4G, adresses publiques refusées) | scaffold (à valider sur Android) |
| Invités : un QR code par ami, permissions au choix, durée limitée, révocation immédiate | scaffold (testé : droits, expiration, révocation) |
| Onglet Mods : mods Workshop de Palworld 1.0 (PalModSettings.ini, téléchargement SteamCMD, activer/désactiver) | scaffold (à valider sur un vrai serveur) |
| Copie des sauvegardes vers un second emplacement, résumé quotidien Discord/ntfy | scaffold fonctionnel (testé) |
| Icône de zone de notification, lancement avec Windows, fermer = réduire | scaffold (à valider sur Windows) |
| Alertes Discord + push mobile (ntfy) : crash, mémoire, connexions, sauvegardes anciennes, disque plein | scaffold fonctionnel (moteur testé) |
| Joueurs : historique, détails (niveau, position, constructions), bannis, liste blanche, sauvegarde de la fiche de chaque joueur | scaffold (à valider sur un vrai monde) |
| Automatisation : cycle de redémarrage sûr avec retour arrière, détection de gel / crashs en boucle, mises à jour auto du serveur et des mods, packs de mods, profils planifiés | scaffold (logique testée, à valider sur un vrai serveur) |
| Bot Discord (/statut, /redemarrer…), installation depuis zéro, UPnP (désactivé par défaut), mise à jour de l'application | scaffold (à valider en réel) |

> ⚠️ Dans un navigateur (`npm run dev`) l'interface affiche des **données fictives** (bandeau « MODE DÉMO »). Seule l'application Tauri pilote un vrai serveur.

## Télécharger l'application (Windows)

Les versions de test sont publiées dans l'onglet **Releases** du dépôt : téléchargez le fichier `…_x64-setup.exe` et lancez-le (l'application n'est pas signée : SmartScreen demande « Informations complémentaires » puis « Exécuter quand même »). Installation pour l'utilisateur courant, sans droits administrateur.

Pour publier une nouvelle version de test : poussez un commit dont le message contient `[release]` sur la branche de travail (ou un tag `v*`) ; le workflow `Release` fabrique l'installeur et crée la release.

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

## Données de l'application

L'installeur ne contient aucune donnée personnelle : réglages, historique, carnet de joueurs et profils sont créés au premier lancement dans `%APPDATA%\dev.palmanager.app`, et les mots de passe/jetons dans le Gestionnaire d'identifiants Windows (entrées « PalworldServerManager »). Au premier lancement, un assistant de configuration s'affiche. Pour repartir de zéro sur un PC : désinstaller, supprimer ce dossier et ces entrées.
