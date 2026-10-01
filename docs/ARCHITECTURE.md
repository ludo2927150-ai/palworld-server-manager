# Architecture

```
┌────────────── Frontend (React + TS + Tailwind, src/) ──────────────┐
│ pages/ Dashboard · Players · Config · Backups · Settings           │
│ lib/api.ts  ── invoke(cmd) / listen("snapshot") ──┐  (mock hors Tauri)
└───────────────────────────────────────────────────┼────────────────┘
                                          IPC Tauri │
┌───────────────────────────────────────────────────▼────────────────┐
│ src-tauri/ (fine couche)                                           │
│  commands.rs    commandes invoke → délèguent au core               │
│  state.rs       AppState (settings, ServerController, Monitor…)    │
│  supervisor.rs  boucle 5 s : mesure → alertes → backup planifié    │
└───────────────────────────────────────────────────┬────────────────┘
┌───────────────────────────────────────────────────▼────────────────┐
│ crates/core (palmanager-core, sans dépendance Tauri, testable seul)│
│  server.rs   start/stop/restart de PalServer.exe                   │
│  rest.rs     client API REST Palworld (players, metrics, save…)    │
│  monitor.rs  sysinfo (CPU/RAM du processus) + REST → Snapshot      │
│  ini.rs      parse/serialize OptionSettings=(...)                  │
│  backup.rs   ZIP de SaveGames, rotation, restauration              │
│  alerts.rs   détection d'événements + Discord / ntfy               │
│  settings.rs configuration de l'app (JSON dans app_config_dir)     │
└────────────────────────────────────────────────────────────────────┘
```

## Choix

- **Tauri 2** plutôt qu'Electron : binaire ~10 Mo, RAM faible (l'app cohabite avec un serveur de jeu gourmand), backend Rust adapté à la gestion de processus.
- **Cœur séparé de Tauri** : la logique se teste avec `cargo test` sans WebView ; la couche Tauri reste triviale.
- **API REST officielle** de Palworld pour joueurs/métriques/save/arrêt propre/kick, plutôt que RCON (déprécié).
- **Push mobile via ntfy.sh** (ou instance auto-hébergée) : aucune app à développer, notifications iOS/Android ; Discord via webhook.

## Flux de données

1. `supervisor` échantillonne toutes les 5 s → `Snapshot` (processus + REST).
2. `AlertEngine.evaluate` compare au snapshot précédent → `Event`s (crash, mémoire ≥ seuil, arrivée/départ) avec cooldown anti-spam ; `expected_stop` empêche une fausse alerte quand l'arrêt vient de l'utilisateur.
3. Crash + `auto_restart` → relance ; backups planifiés : `save` REST puis ZIP puis rotation.
4. Le snapshot est émis au frontend (événement `snapshot`).

## Chemins Palworld (Windows)

- Exécutable : `<server_dir>\PalServer.exe` (le vrai processus est `PalServer-Win64-Shipping-Cmd.exe`, détecté par le monitor).
- Config : `<server_dir>\Pal\Saved\Config\WindowsServer\PalWorldSettings.ini`
- Mondes : `<server_dir>\Pal\Saved\SaveGames`

## Sécurité

- Secrets (mot de passe admin, webhooks, jeton mobile, jetons d'invités) : sur Windows ils sont déplacés dans le Gestionnaire d'identifiants (`core::secrets`, crate `keyring`) et remplacés par `@secret` dans `settings.json` ; migration automatique au premier lancement. Si le magasin est indisponible ou refuse l'écriture, la valeur reste en clair (jamais de perte). Les sauvegardes de `settings.json` ne contiennent donc plus les secrets : ils sont liés au compte Windows.
- Restauration : chemin limité au dossier de backup, extraction protégée contre le zip-slip, ancien monde conservé en `SaveGames.bak`.
- `write_world_settings` crée un `.ini.bak` avant écrasement.

## Accès mobile (crates/core/src/remote.rs)

- Serveur HTTP (axum) embarqué, **désactivé par défaut**, qui sert `mobile.html` (page publique, sans donnée) et une API JSON sous `/api`.
- Toute l'API exige `Authorization: Bearer <jeton>` (128 bits, comparaison en temps constant, 300 ms de délai sur échec).
- Filtre d'adresses : seules les connexions venant de la boucle locale, des réseaux privés, du lien local et de Tailscale (100.64.0.0/10) sont acceptées ; une adresse publique est refusée **même avec le bon jeton**.
- Accès hors du domicile : Tailscale (réseau privé chiffré) ; aucune redirection de port sur la box.
- Exposé : état, historique, journal, démarrer/arrêter/redémarrer, sauvegarde, annonce, expulsion. Non exposé : réglages (secrets), configuration du monde, restauration, mise à jour, bannissement.
- Le routeur est générique sur le trait `Backend`, implémenté par `src-tauri/src/remote.rs` : testable sans Tauri.

### Invités

- `settings.remote.guests` : une clé par invité, avec `perms` (status, players, logs, charts, start, stop, restart, backup, announce, kick) et une expiration optionnelle.
- `Credentials` (partagé, modifiable à chaud) : une invitation créée ou révoquée prend effet immédiatement, sans relancer l'écoute.
- Chaque route vérifie le droit requis (403 sinon) ; `snapshot` masque les pseudos sans le droit `players` ; `/api/me` indique à la page mobile ce qu'elle peut afficher.
- Les clés d'invités sont stockées en clair dans `settings.json` (comme celle du propriétaire) ; migration vers le Credential Manager prévue.

## Fiabilité et automatisation

- **Archives** (`backup::verify`) : lecture complète (CRC), refus des archives vides/tronquées/dangereuses. Appelée par « Vérifier », et **avant** d'arrêter le serveur ou de toucher au monde dans une restauration.
- **Santé des sauvegardes** (`health.rs`) : alerte si la dernière sauvegarde réussie est trop ancienne (serveur en marche) ou si le disque de destination est presque plein ; une alerte par type et par 6 h.
- **Mise à jour du serveur** : `steamcmd::installed_build` (manifeste `appmanifest_2394010.acf`) comparé à `latest_build` (`app_info_print`). Une seule tentative par build ; préavis aux joueurs ; échec = alerte, jamais de boucle.
- **Profils** (`profiles.rs`) : jeux de réglages du monde ; l'application ne modifie jamais mots de passe, ports ni API REST.
- **Bot Discord** (`discord.rs`) : passerelle Discord en connexion sortante (aucun port ouvert), commandes slash, liste blanche d'identifiants Discord (vide = personne), contrôle désactivé par défaut (lecture seule), jeton dans le Gestionnaire d'identifiants. Gère le délai de 3 s de Discord par réponse différée.

## Automatisation (src-tauri/src/automation.rs, supervisor.rs)

- **Cycle sûr** (`guarded_cycle`) : sauvegarde de sûreté → arrêt propre → changement (profil, mise à jour SteamCMD) → démarrage → contrôle (processus vivant + API REST) pendant 6 min. Sinon : (1) désactivation des mods suspects (ou de tous), (2) restauration du monde depuis la sauvegarde de sûreté + réglages de mods d'origine. Tous les redémarrages automatiques (horaires, mémoire, gel, mises à jour, mods) passent par là. Une mise à jour de binaire du serveur ne peut pas être annulée (SteamCMD) : le retour arrière protège le monde et les mods.
- **Surveillance** (`core/watchdog.rs`, logique pure) : gel = API muette N min après avoir répondu au moins une fois (jamais si l'API est désactivée) ; boucle de crashs = X crashs en Y min → relance auto arrêtée jusqu'à un démarrage manuel.
- **Mods gérés** : `managed_ids` téléchargés et vérifiés par empreinte de dossier (`mods::dir_signature`) ; un changement sur un serveur en marche déclenche un cycle sûr avec préavis. Packs = ensembles de mods activables d'un clic (`mods::apply_pack`).
- **Planning + profils** : une règle peut appliquer un profil avant son exécution (« XP ×3 le week-end »).
- **Sauvegardes** : rétention par paliers (`backup::plan_tiered`) en option ; chaque archive est relue (`verify`) juste après création, supprimée et signalée si défectueuse.
- **Installation depuis zéro** (`core/install.rs`) : seule adresse téléchargée = steamcmd.zip officiel de Valve.
- **UPnP** (`core/upnp.rs`) : ouvre uniquement le port UDP de jeu, désactivé par défaut, bail d'1 h renouvelé toutes les 20 min tant que le serveur tourne.
