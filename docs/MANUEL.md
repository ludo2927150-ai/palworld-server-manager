# Palworld Server Manager — Manuel d'utilisation

Ce manuel décrit **toutes** les fonctions du logiciel, onglet par onglet, avec les réglages, les automatismes, les alertes, les fichiers utilisés, des recettes pratiques et un guide de dépannage. Il est écrit pour l'utilisateur, pas pour le développeur (l'architecture technique est dans `docs/ARCHITECTURE.md`).

> **État du logiciel.** Il s'agit d'une version de test. Une grande partie de la logique est vérifiée par des tests automatiques, mais certaines fonctions n'ont **pas** encore été éprouvées sur un vrai serveur Palworld : elles sont signalées par « ⚠ non vérifié en réel » et listées au chapitre 24.

---

## Table des matières

1. [Présentation](#1-présentation)
2. [Installation et mises à jour](#2-installation-et-mises-à-jour)
3. [Premier lancement : l'assistant](#3-premier-lancement--lassistant)
4. [Tour de l'interface](#4-tour-de-linterface)
5. [Tableau de bord](#5-tableau-de-bord)
6. [Rejoindre](#6-rejoindre)
7. [Historique](#7-historique)
8. [Performance](#8-performance)
9. [Mods](#9-mods)
10. [Joueurs](#10-joueurs)
11. [Annonces](#11-annonces)
12. [Configuration du monde](#12-configuration-du-monde)
13. [Journal](#13-journal)
14. [Sauvegardes](#14-sauvegardes)
15. [Diagnostic](#15-diagnostic)
16. [Mobile et invités](#16-mobile-et-invités)
17. [Application (réglages généraux)](#17-application-réglages-généraux)
18. [Les automatismes expliqués](#18-les-automatismes-expliqués)
19. [Alertes et notifications](#19-alertes-et-notifications)
20. [Bot Discord](#20-bot-discord)
21. [Sécurité et vie privée](#21-sécurité-et-vie-privée)
22. [Où sont rangés les fichiers](#22-où-sont-rangés-les-fichiers)
23. [Recettes pratiques](#23-recettes-pratiques)
24. [Limites connues et points non vérifiés](#24-limites-connues-et-points-non-vérifiés)
25. [Dépannage](#25-dépannage)
26. [Glossaire](#26-glossaire)
27. [Annexes (tableaux de référence)](#27-annexes-tableaux-de-référence)

---

## 1. Présentation

### 1.1 À quoi sert le logiciel

Palworld Server Manager est une application Windows qui pilote **un** serveur dédié Palworld : le démarrer, l'arrêter, le surveiller, le sauvegarder, le mettre à jour, le configurer et le réparer automatiquement, depuis le PC ou depuis un téléphone.

Il ne remplace pas le jeu ni le serveur : il les **pilote** en combinant trois sources :

| Source | Ce qu'elle apporte |
|---|---|
| **L'API REST officielle du serveur** (port 8212, mot de passe admin) | Liste des joueurs, FPS, niveau, position, annonces, expulsion, bannissement, sauvegarde, arrêt propre |
| **Les fichiers du serveur** (`PalWorldSettings.ini`, `SaveGames`, `Mods`…) | Configuration, sauvegardes, mods |
| **Le système Windows** | Processus du serveur, CPU/RAM, priorité, démarrage avec Windows, notifications |
| **SteamCMD** (outil officiel de Valve) | Installation et mises à jour du serveur, téléchargement des mods |

### 1.2 Ce qu'il fait tout seul

Une fois réglé, il peut gérer le serveur sans vous : sauvegardes régulières et à chaque arrêt, redémarrages programmés avec annonces, relance après un crash, détection d'un serveur gelé, mises à jour du serveur et des mods, événements saisonniers, alertes Discord/téléphone, rapports. Tout redémarrage automatique est suivi d'un **contrôle** et d'un **retour arrière** si le serveur ne repart pas (chapitre 18).

### 1.3 Ce dont vous avez besoin

- Windows 10 ou 11 (64 bits).
- Le serveur dédié Palworld (l'application peut l'installer pour vous, chapitre 3).
- Environ 12 Go d'espace libre pour une installation complète du serveur.
- Facultatif : un compte Discord (alertes, bot), le service gratuit Tailscale (accès depuis le téléphone hors de chez vous).

### 1.4 Vocabulaire minimal

- **Serveur** : le programme `PalServer.exe` (le jeu en mode serveur).
- **Application** : Palworld Server Manager (ce logiciel).
- **Monde** : les données de la partie (`SaveGames`).
- **API REST** : la « porte » de service du serveur que l'application utilise. Elle ne doit **jamais** être ouverte sur Internet.
- Voir aussi le [glossaire](#26-glossaire).

---

## 2. Installation et mises à jour

### 2.1 Installer

1. Ouvrez la page **Releases** du dépôt GitHub du projet et téléchargez le fichier se terminant par `_x64-setup.exe`.
2. Double-cliquez dessus. L'installation se fait **pour l'utilisateur courant**, sans droits administrateur.
3. Windows SmartScreen peut afficher « Windows a protégé votre ordinateur » : le programme n'est pas signé (une signature de code est payante). Cliquez sur **Informations complémentaires**, puis **Exécuter quand même**.
4. Lancez « Palworld Server Manager » depuis le menu Démarrer.

### 2.2 Où vont vos données

L'installeur ne contient **aucune** donnée personnelle. Au premier lancement, l'application crée :
- ses réglages, son historique et ses sauvegardes dans `%APPDATA%\dev.palmanager.app` ;
- vos mots de passe et jetons dans le **Gestionnaire d'identifiants de Windows** (entrées « PalworldServerManager »).

Détail au [chapitre 22](#22-où-sont-rangés-les-fichiers).

### 2.3 Mises à jour de l'application

Quelques secondes après le lancement, puis toutes les 6 heures, l'application consulte les Releases GitHub du projet (lecture publique, aucune donnée envoyée). Si une version plus récente existe, un **bandeau** apparaît en haut :

> Nouvelle version disponible : X (vous avez Y). **[Télécharger et installer]** — Plus tard

« Télécharger et installer » télécharge l'installeur (seules les adresses de release du dépôt sont acceptées), le lance, puis ferme l'application. Vos réglages sont conservés.

### 2.4 Désinstaller / repartir de zéro

- **Désinstaller** : Paramètres Windows → Applications.
- **Repartir de zéro** : après désinstallation, supprimez le dossier `%APPDATA%\dev.palmanager.app` et les entrées « PalworldServerManager » du Gestionnaire d'identifiants. Votre serveur Palworld et vos mondes ne sont pas touchés.

### 2.5 Mode développement (pour tester les changements en direct)

Le fichier `Lancer-Palworld-Manager.bat` (à la racine du dépôt cloné) lance l'application en mode développement et récupère automatiquement les mises à jour du code toutes les 10 secondes. Ce mode est réservé au développement : « Lancer avec Windows » y est volontairement refusé (une version de développement afficherait une console noire).

---

## 3. Premier lancement : l'assistant

Au premier lancement, une fenêtre **« Bienvenue — configuration en 5 étapes »** s'affiche. Le bouton **Passer** (en haut à droite) la ferme à tout moment ; vous pourrez tout régler plus tard.

### Étape 1 — Dossier du serveur
- Le bloc **« Pas encore de serveur ? Tout installer automatiquement »** télécharge SteamCMD puis le serveur Palworld (plusieurs Go) dans le dossier choisi (par défaut `C:\palworld`) et configure l'application. Il refuse de démarrer s'il reste moins de 12 Go libres. Une zone de texte affiche la progression.
- Sinon, choisissez le dossier détecté (les installations usuelles sont repérées automatiquement) ou saisissez le chemin du dossier qui contient `PalServer.exe`.

### Étape 2 — SteamCMD
SteamCMD sert aux mises à jour du serveur et au téléchargement des mods. Choisissez le chemin détecté ou saisissez celui de `steamcmd.exe`.

### Étape 3 — API REST
L'application pilote le serveur par son API REST. Saisissez un **mot de passe admin** et cliquez **Appliquer** : il est écrit dans `PalWorldSettings.ini` (une copie `.bak` est créée), l'API REST est activée, et le mot de passe est mémorisé par l'application. Le serveur doit être redémarré pour que le changement s'applique.

### Étape 4 — Options
Cases à cocher : sauvegardes automatiques (et intervalle), relance après un crash, démarrage du serveur au lancement de l'application, fermer la fenêtre = réduire dans la zone de notification.

### Étape 5 — Vérification
« Lancer le diagnostic » contrôle l'installation. « API REST joignable » ne devient vert qu'une fois le serveur démarré. **Terminer** enregistre.

---

## 4. Tour de l'interface

### 4.1 Le menu de gauche

Treize onglets : Tableau de bord, Rejoindre, Historique, Performance, Mods, Joueurs, Annonces, Configuration, Journal, Sauvegardes, Diagnostic, Mobile, Application. Sous le menu : le sélecteur de **Thème** (Sombre / Clair) et de **Langue** (Français / English).

### 4.2 Bandeaux en haut de la fenêtre

| Bandeau | Signification |
|---|---|
| **MODE DÉMO** (orange) | Vous êtes dans un navigateur, pas dans l'application : les données sont fictives. |
| **Nouvelle version disponible** | Une mise à jour de l'application existe (§ 2.3). |

### 4.2 bis Boutons en cours d'exécution

Quand vous cliquez sur un bouton qui lance une tâche (démarrer, arrêter, redémarrer, sauvegarder, restaurer, vérifier, enregistrer, tester…), **un petit rond tournant** apparaît sur le bouton tant que la tâche n'est pas terminée. Il disparaît dès qu'elle est finie, qu'elle ait réussi ou échoué (le résultat s'affiche alors dans un message). Pendant ce temps, le bouton ignore les clics supplémentaires : impossible de lancer deux fois la même action par erreur. La page Mobile fonctionne de la même façon.

### 4.3 Zone de notification (barre des tâches)

L'application met une icône près de l'horloge. Clic droit : **Ouvrir Palworld Manager** / **Quitter**. Avec « Fermer la fenêtre = réduire dans la zone de notification » (chapitre 17), la croix ne quitte pas l'application : elle la cache, ce qui permet de garder la surveillance active. **Pour arrêter vraiment l'application : clic droit sur l'icône → Quitter.**

> Les automatismes (sauvegardes, horaires, relances, alertes) ne fonctionnent **que si l'application tourne**. Si vous la quittez, plus rien n'est surveillé.

### 4.4 Verrou par code

Si vous avez défini un code (chapitre 17), un écran « Application verrouillée » s'affiche au lancement et après le délai d'inactivité choisi. Entrer un mauvais code ralentit la réponse d'une seconde.

### 4.5 Langue et thème

- **Thème clair** : complet, il suit toute l'interface.
- **English** : traduction **partielle** (navigation, bandeaux, assistant, page Joueurs). Le reste reste en français.

---

## 5. Tableau de bord

La page d'accueil. En haut, l'état (« Serveur en ligne » / « Serveur arrêté ») et quatre boutons :

| Bouton | Effet |
|---|---|
| **Démarrer** | Lance le serveur. Si l'option est active, l'application capte sa console (voir § 13.2). |
| **Redémarrer** | Arrêt propre (API REST), sauvegarde, puis démarrage. |
| **Mettre à jour** | Cycle sûr de mise à jour via SteamCMD (§ 18.1) : sauvegarde, arrêt, mise à jour, démarrage, contrôle, retour arrière si besoin. |
| **Arrêter** | Arrêt propre, avec sauvegarde. L'application n'interprète pas cet arrêt comme un crash. |

Quatre cartes : **CPU** (%), **Mémoire** (Go et barre), **Joueurs** (connectés / maximum), **FPS serveur** (et le jour de jeu).

En dessous : le **journal du serveur en direct** (§ 13.2).

> « Arrêter » demande l'arrêt à l'API REST (avec un décompte de 10 secondes affiché aux joueurs), attend l'arrêt réel jusqu'à 60 secondes, puis force l'arrêt si besoin.

---

## 6. Rejoindre

Cette page rassemble tout ce qu'il faut donner à vos amis.

- **Adresse sur le réseau local** (même Wi-Fi/box) : l'adresse du PC suivie du port du jeu.
- **Adresse depuis Internet** : bouton qui interroge le service `api.ipify.org` **une seule fois, au clic**, pour afficher votre IP publique.
- **Mot de passe du serveur** : affiché s'il existe. Un avertissement apparaît s'il n'y en a pas (« toute personne connaissant l'adresse peut entrer »).
- **Message à envoyer à vos amis** : un texte prêt à copier (bouton **Copier le message**).
- **Où saisir ces informations dans le jeu** : lancez Palworld, choisissez le menu multijoueur, puis saisissez `adresse:port` dans le champ en bas de la liste des serveurs.
- **Pour que vos amis entrent depuis Internet** : sur votre box, redirigez le port **8211 en UDP** vers le PC (ou activez l'UPnP, § 17.9), et autorisez le serveur dans le pare-feu Windows. **Ne redirigez jamais le port 8212** (API REST) : il sert uniquement à l'application.

Les boutons **Copier** copient chaque information.

---

## 7. Historique

Quatre graphiques (24 h par défaut, jusqu'à 7 jours de données conservées) : **CPU**, **Mémoire**, **Joueurs connectés**, **FPS serveur**. Un échantillon est enregistré toutes les 30 secondes tant que le serveur tourne.

Dessous : **Sessions de jeu (7 derniers jours)** — joueur, heure d'arrivée, durée. Une session sans heure de fin signifie « toujours connecté ».

Cet historique alimente le résumé quotidien et le rapport hebdomadaire (chapitre 19). Les événements de connexion sont gardés 30 jours.

---

## 8. Performance

Cette page vous donne la main sur les ressources du serveur.

### 8.1 Surveillance
Jauges **RAM du serveur** et **CPU du serveur**, rappel du nombre de **cœurs logiques** et de la RAM totale du PC.

### 8.2 Mémoire (RAM)
**Limite de RAM du serveur (Go)** — vide = aucune. Quand le serveur dépasse la limite :
- vous êtes alerté ;
- si **« redémarrer »** est coché, le serveur est redémarré après 1 minute de préavis (annoncé aux joueurs présents), au plus **une fois toutes les 30 minutes** (pour éviter une boucle si la limite est trop basse).

Le redémarrage passe par le cycle sûr (§ 18.1).

### 8.3 Processeur (CPU)
- **Priorité du serveur** : Basse, Normale (défaut de Windows), Haute (recommandée si le PC fait aussi autre chose), Très haute (peut ralentir le reste du PC). La priorité « temps réel » n'est volontairement pas proposée (elle peut figer Windows).
- **Cœurs** : cochez les cœurs que le serveur peut utiliser ; le bouton **« Laisser 2 cœurs à Windows »** applique un réglage prudent. Un masque vide est impossible : le serveur garde toujours au moins un cœur.

Priorité et cœurs sont **réappliqués à chaque démarrage du serveur** (et à chaque changement de réglage). Si rien ne change, lancez l'application en administrateur. ⚠ non vérifié en réel.

### 8.4 Options de lancement (threads)
Cases pour `-useperfthreads`, `-NoAsyncLoadingThread`, `-UseMultithreadForDS` et un champ **Threads de travail du serveur** (`-NumberOfWorkerThreadsServer`, vide = automatique). Elles ne s'appliquent qu'au **prochain démarrage** du serveur. L'encadré rappelle les arguments actuels.

### 8.5 Réglages du jeu qui allègent le serveur
Pense-bête vers l'onglet Configuration : `bEnableInvaderEnemy` (désactivé = moins de fuite mémoire), `ServerReplicatePawnCullDistance`, `BaseCampWorkerMaxNum`, `DropItemMaxNum`, `MaxBuildingLimitNum`.

Boutons **Enregistrer** et **Annuler les changements**.

---

## 9. Mods

L'onglet gère les **mods du Steam Workshop pour Palworld 1.0** (chargeur officiel de Pocketpair) : l'application écrit pour vous le fichier `Mods\PalModSettings.ini`, sans perte des lignes qu'elle ne connaît pas (une copie `.bak` est conservée).

> ⚠ Un mod est du **code tiers exécuté sur votre serveur**. N'installez que des mods dont vous avez lu la page Workshop. Seuls les mods marqués « compatible serveur » fonctionnent sur un serveur dédié.

### 9.1 Ajouter un mod
Collez l'**identifiant** du mod (ou l'adresse de sa page Workshop) puis validez. L'application le télécharge via SteamCMD **dans le dossier du serveur**, puis :
- le déclare « **géré** » (il sera tenu à jour automatiquement si vous activez la mise à jour automatique, § 17.8) ;
- l'**active** s'il est compatible serveur.

Un bandeau **« Les mods sont lus au démarrage du serveur : redémarrez-le »** apparaît, avec le bouton **Redémarrer maintenant**.

Certains mods refusent le téléchargement anonyme : abonnez-vous alors au mod depuis le **client Steam** (sur ce PC) et choisissez le dossier Workshop de Steam.

### 9.2 Liste des mods
Pour chaque mod détecté : nom, version, auteur, compatibilité serveur, état actif/inactif (interrupteur), lien **Page Workshop**, bouton **Supprimer** (uniquement pour ceux téléchargés par l'application ; ceux du client Steam se retirent depuis Steam).

### 9.3 Réglages globaux
- Interrupteur général des mods (`bGlobalEnableMod`).
- **Dossier Workshop lu par le serveur** : par défaut celui du dossier du serveur ; vous pouvez en choisir un autre (il doit exister). Gardez **un seul** dossier Workshop pour éviter les incohérences.
- `bAllowClientMod` : rappel du réglage qui autorise ou non les joueurs à jouer avec des mods clients.

### 9.4 Packs de mods
En haut de la page : enregistrez l'ensemble des mods actifs sous un nom (« Vanilla », « Avec mods »…) et **basculez en un clic** (**Appliquer**). Un pack n'active que les mods présents et compatibles serveur ; les manquants sont signalés. **Remplacer par l'actuel** met le pack à jour ; **Supprimer** le retire. Un redémarrage du serveur est nécessaire.

### 9.5 Mises à jour des mods
Un mod téléchargé par l'application se met à jour en étant retéléchargé (automatiquement si l'option est active) ; ceux du client Steam se mettent à jour par Steam. Quand un mod change sur un serveur en marche, l'application déclenche un **redémarrage sûr** avec préavis (§ 18.1).

---

## 10. Joueurs

Quatre sous-onglets : **En ligne**, **Historique**, **Bannis**, **Liste blanche**.

### 10.1 En ligne
Liste des joueurs connectés : nom, niveau, ping. Actions :
- **Expulser** : le joueur est déconnecté (il peut revenir).
- **Bannir** : demande une **raison** (facultative), bannit via l'API REST et l'enregistre dans le carnet de l'application.
- Zone **Annonce à tous les joueurs** : envoie un message en jeu.

### 10.2 Historique
Le **carnet de tous les joueurs déjà vus**. Il se remplit tant que le serveur tourne **et que l'application est ouverte**. Pour chacun : état (en ligne, banni, autorisé), dernière vue, **temps de jeu**, nombre de sessions, première vue, anciens pseudos (5 au plus). Un champ **Rechercher** filtre par pseudo ou identifiant. Boutons par ligne : **Détails**, **Autoriser** (ajoute à la liste blanche), **Bannir / Débannir**.

> Le temps de jeu ne compte que les présences observées en continu par l'application. Une absence de plus de 30 secondes entre deux observations n'est pas comptée.

### 10.3 Fiche détaillée d'un joueur (bouton « Détails »)

| Information | Détail |
|---|---|
| Niveau | Actuel, et maximum atteint |
| Progression | Courbe du niveau dans le temps |
| Constructions | Nombre de constructions (si l'API le fournit) |
| Dernière position | Coordonnées du jeu X / Y |
| Temps de jeu, sessions | Cumul observé |
| Première et dernière vue | Dates |
| Anciens pseudos | Historique des changements |
| Identifiant de compte | `steam_…` (pour la liste blanche, les bannissements) |
| Identifiant joueur | 32 caractères, nom des fichiers `Players\<id>.sav` |

Les niveaux, positions et constructions viennent de l'API REST : ils se remplissent quand le joueur est connecté **application ouverte**. Je ne lis ni ne conserve l'adresse IP des joueurs.

### 10.4 Sauvegardes d'un joueur (dans la fiche)

À chaque sauvegarde du monde, la **fiche de chaque joueur** (ses fichiers `Players\<id>.sav` et `_dps.sav`) est copiée à part, dans `<dossier de sauvegarde>\joueurs\<ID>\`, au plus **10 versions** par joueur (réglable), sans doublon si rien n'a changé.

Dans la fiche : **Sauvegarder maintenant**, **Exporter** (copie le zip vers un dossier choisi, par défaut le Bureau), **Restaurer cette fiche**.

**Restaurer une fiche** : arrête le serveur (après sauvegarde du monde), remplace les fichiers du joueur, relance le serveur s'il tournait. L'ancien fichier est conservé en `.sav.avant-restauration`. Refusé si le monde de la sauvegarde n'existe pas sur ce serveur.

> **Limite essentielle.** La fiche contient le niveau, les statistiques, les technologies, la position… mais **les objets, les Pals et les bases du joueur sont dans le fichier du monde (`Level.sav`)**. Restaurer une fiche ne les ramène donc pas. Pour récupérer des objets ou des Pals perdus, il faut revenir à une **sauvegarde du monde entier** (chapitre 14). Transférer un joueur vers **un autre serveur** n'est pas possible sans réécrire `Level.sav` (format fermé) : l'application ne le fait pas, pour ne jamais corrompre un monde. L'export zip sert à alimenter un outil communautaire, à vos risques.

### 10.5 Bannis
Liste des bannissements faits **depuis l'application** (nom, identifiant, date, raison) avec **Débannir**. Un champ permet de débannir par identifiant (`steam_…`). Le serveur garde aussi sa propre liste, que l'application ne lit pas.

### 10.6 Liste blanche
Interrupteur **Activée (les autres sont expulsés)** : tout joueur absent de la liste est **expulsé quelques secondes après sa connexion** (avec le message que vous choisissez, par défaut « Ce serveur est privé (liste blanche). »).

Garde-fous :
- **Une liste vide n'expulse personne** (sinon tout le monde serait dehors).
- Elle ne fonctionne que si l'application tourne **et** que l'API REST répond.
- Un joueur dont l'identifiant est illisible n'est jamais expulsé.
- Une même personne n'est expulsée qu'une fois toutes les 20 secondes.

Ajout : **Ajouter les joueurs connectés**, ou saisie d'un identifiant, ou **Autoriser** depuis l'historique. **Retirer** enlève un joueur.

---

## 11. Annonces

Messages envoyés à **tous les joueurs** via l'API REST. Interrupteur général **Activées**, et case **N'envoyer les rappels que si au moins un joueur est connecté**.

Variables utilisables dans les textes : `{nom}` (le joueur qui arrive), `{joueurs}` (connectés), `{max}` (places), `{jours}` (absence, message de retour).

### 11.1 Message de bienvenue
Envoyé quand un joueur se connecte (300 caractères au plus ; les caractères de contrôle sont retirés). **Les joueurs déjà présents au lancement de l'application ne sont pas accueillis.**

### 11.2 Messages personnels et retours
- **Message personnel** : un joueur précis, un texte propre à chaque connexion, et une **date d'anniversaire** `MM-JJ`. Il **remplace** le message général pour ce joueur.
- **Retour après absence** : après N jours sans venir (0 = désactivé), un message spécial (`{nom}`, `{jours}`).

Ordre de priorité à la connexion : **anniversaire** → **message personnel** → **retour après absence** → **bienvenue générale**.

### 11.3 Rappels réguliers
Une liste de textes envoyés toutes les N minutes (1 à 1440), chacun activable. Un rappel ne part qu'**après un intervalle complet** (jamais au lancement de l'application) et pas pendant une maintenance. Bouton **Tester** : l'envoie immédiatement.

Boutons **Enregistrer** / **Annuler**.

---

## 12. Configuration du monde

Éditeur graphique de `PalWorldSettings.ini` (environ 90 réglages, lecture et écriture fidèles). **Redémarrez le serveur** après un enregistrement.

### 12.1 Groupes de réglages
| Groupe | Contenu (exemples) |
|---|---|
| **Général** | Nom, description, mot de passe, mot de passe admin, joueurs max |
| **Réseau** | Port du jeu, IP publique, API REST (activée, port), RCON |
| **Taux** | XP, capture, apparition des Pals, vitesse du jour/nuit, vitesse de travail |
| **Combat** | PvP, dégâts joueur/joueur, tirs alliés, raids, dégâts des Pals |
| **Survie** | Pénalité de mort, hardcore, perte des Pals, faim, endurance, régénération |
| **Bases & guildes** | Bases max, travailleurs par base, guildes, inactivité |
| **Système** | Sauvegarde auto, objets au sol, largages, distance de réplication |
| **Avancé** | Toute clé du fichier qui n'a pas d'écran dédié |

### 12.2 Préréglages, import, export
- **Préréglages** : *Décontracté*, *Standard*, *Difficile*. Un clic remplit les valeurs (**sans enregistrer**).
- **Exporter** / **Importer** : sauvegarde ou charge la configuration au format JSON.

### 12.3 Enregistrer en sûreté
**Enregistrer (n)** ouvre un **récapitulatif des différences** (ancienne et nouvelle valeur) à valider. Une copie `.ini.bak` est créée avant d'écrire. **Annuler les changements** restaure l'affichage. Un nom contenant un guillemet `"` ou un saut de ligne est **refusé** (il casserait le fichier).

Si `PalWorldSettings.ini` est vide (serveur jamais configuré), un bandeau l'indique : les valeurs viennent de `DefaultPalWorldSettings.ini` et l'enregistrement crée le fichier.

### 12.3 bis Profils de configuration
Bloc **Profils de configuration** en haut de la page : enregistrez les réglages actuels du monde sous un nom (« Normal », « Hardcore », « Soirée ») et rebasculez en un clic (**Appliquer** : copie `.bak` conservée, redémarrage nécessaire).

Un profil **ne modifie jamais** les mots de passe (`AdminPassword`, `ServerPassword`), les ports, ni l'API REST/RCON, ni `PublicIP`/`Region`. Attention : « Enregistrer sous ce nom » enregistre le fichier **tel qu'il est sur le disque**, pas des changements non enregistrés (un avertissement s'affiche).

Les profils servent aussi aux **horaires** (§ 17.4) et au **calendrier de saisons** (§ 17.5).

---

## 13. Journal

La page a trois blocs : l'analyse, le journal du serveur et le journal d'audit.

### 13.1 Analyse du journal
Analyse la fin du journal (≈ 64 Ko) et affiche les **causes probables** de problèmes, avec un conseil :

| Cause détectée | Gravité |
|---|---|
| Mémoire insuffisante | Critique |
| Plantage du moteur (erreur fatale) | Critique |
| Disque plein | Critique |
| Port déjà utilisé | Critique |
| Problème d'écriture ou de lecture du monde | Critique |
| Erreur liée à un mod | Avertissement |
| API REST indisponible | Avertissement |
| Version du jeu différente | Avertissement |
| Lignes d'erreur diverses | Info |

En cas de crash, la cause probable est ajoutée au message d'alerte.

### 13.2 Journal du serveur (aussi sur le tableau de bord et le mobile)
Lecture en direct (toutes les 2 secondes) avec **filtre**, case **Suivre** (défilement automatique) et case **Masquer les appels REST** (cochée par défaut : retire les lignes `REST accessed endpoint…` produites par l'application elle-même toutes les 5 s).

**D'où viennent les lignes.** Ce serveur écrit sa console dans sa fenêtre, pas toujours dans un fichier. L'application procède ainsi :
1. Quand **elle démarre** le serveur (bouton Démarrer, relance auto, horaires), elle lance le moteur dans une **console virtuelle** (ConPTY) et enregistre sa sortie dans `server-console.log` (5 Mo au plus, l'ancien devient `.old`). Aucune fenêtre noire n'apparaît.
2. Elle lit **le fichier journal le plus récent** parmi `Pal\Saved\**.log` et `server-console.log`. Le nom du fichier lu est affiché sous le filtre.
3. Vous pouvez imposer un fichier dans **Application → Fichier journal du serveur**.

> Un serveur **lancé à la main, avant l'application**, ne peut pas être capté : arrêtez-le et relancez-le avec **Démarrer**. Si aucun journal n'est disponible, l'écran l'explique et indique ce qu'il a trouvé. L'option « Afficher la console du serveur dans l'application » (chapitre 17) se désactive en cas de souci ; l'application se replie alors sur l'ancien lancement. ⚠ non vérifié en réel.

### 13.3 Journal d'audit
Tableau **Quand / Qui / Action** des actions faites sur le serveur, conservées **90 jours**. Un champ filtre. Le « Qui » :

| Valeur | Origine |
|---|---|
| Application | Vous, depuis cette fenêtre |
| Mobile · *nom* | Un téléphone (propriétaire ou invité) |
| Discord · *identifiant* | Une commande du bot Discord |
| Automatique · *étiquette* | Un automatisme (`maj`, `mods`, `gel`, `memoire`, `planifie`, `saison`, `test-restauration`) |

Exemples d'actions journalisées : démarrage/arrêt/redémarrage, sauvegarde manuelle ou protégée, restauration, mise à jour, profil appliqué, modification de la configuration, mod ajouté/retiré, annonce, expulsion, bannissement, code du verrou refusé.

---

## 14. Sauvegardes

### 14.1 Principe
Une sauvegarde est une archive **ZIP** du dossier `SaveGames` (le monde). Chaque archive est **relue juste après sa création** : si elle est illisible, elle est supprimée et une alerte est envoyée (une archive qu'on ne peut pas relire est pire que pas d'archive).

### 14.2 Quand une sauvegarde est créée
| Déclencheur | Étiquette dans le nom |
|---|---|
| Intervalle régulier (serveur en marche) | `-auto` |
| À chaque arrêt ou redémarrage voulu | `-arret` |
| Arrêt non voulu (fenêtre fermée, crash), avant toute relance | `-arret-externe` |
| Bouton « Sauvegarder maintenant » | `-manuel` |
| Juste avant une restauration | `-avant-restauration` |
| Avant une maintenance risquée (mise à jour, mods, profil, saison) | `-avant-maj`, `-avant-mods`… |
| Bouton « Sauvegarde protégée » | `-garde` ou `-garde-<nom>` |

Avant chaque sauvegarde, l'application demande au serveur d'écrire son monde sur disque (`save`) pour obtenir une copie cohérente.

### 14.3 Réglages (haut de la page)
- **Sauvegarde automatique pendant que le serveur tourne** + **Intervalle (minutes)** (s'applique immédiatement).
- **Sauvegarde à chaque arrêt ou redémarrage** du serveur.
- **Nombre de sauvegardes conservées** (les plus anciennes sont supprimées).
- Autres réglages dans **Application** : rétention par paliers, dossier de destination, **second emplacement**, versions par joueur, test de restauration.

### 14.4 Rétention par paliers (option)
Au lieu d'un nombre fixe : tout ce qui a moins de **24 h**, puis **une par jour** pendant 7 jours, puis **une par semaine** pendant 4 semaines. La toute dernière sauvegarde est toujours conservée.

### 14.5 Sauvegardes protégées
**Sauvegarde protégée** (bouton) crée une archive que **la rotation ne supprime jamais** (avant un boss, un raid, une grosse modification). Sur chaque archive, **Protéger / Déprotéger** ajoute ou retire `-garde` du nom. Les archives protégées ne comptent pas dans le nombre conservé.

### 14.5 bis Second emplacement
Chaque sauvegarde est aussi copiée dans un **second dossier** (autre disque, dossier synchronisé OneDrive/Dropbox…), avec la même politique de rotation. Si la copie échoue, vous êtes alerté (au plus une fois par heure).

### 14.6 Sauvegarde interne du jeu (AutoSaveSpan)
Bloc **Sauvegarde interne du jeu** : réglage, en minutes (0,5 à 120), de l'intervalle auquel le **serveur** écrit lui-même le monde (`AutoSaveSpan`). Plus c'est court, moins on perd en cas de crash. Écrit dans `PalWorldSettings.ini` ; redémarrage nécessaire.

### 14.7 Liste et actions
Chaque ligne : date, taille, origine, badges **la plus récente** et **protégée**, et :
- **Protéger / Déprotéger** ;
- **Vérifier** : lit l'archive complète (contrôle des sommes de contrôle) sans rien restaurer ;
- **Revenir à celle-ci** : restauration complète.

### 14.8 Revenir à une sauvegarde (restauration)
Bouton **Revenir à celle-ci**, après confirmation :
1. L'archive est d'abord **vérifiée** (avant d'arrêter quoi que ce soit : une archive douteuse n'écrase jamais un bon monde) et copiée en lieu sûr.
2. Si le serveur tourne, il est **arrêté** (une sauvegarde de l'état actuel est faite).
3. Le monde actuel est déplacé en `SaveGames.bak`, puis l'archive est extraite.
4. Le serveur est **relancé** s'il tournait.

Rien n'est perdu : l'état d'avant reste en `-arret` / `-avant-restauration` et en `SaveGames.bak`. Seuls les fichiers du dossier de sauvegarde configuré sont acceptés.

### 14.9 Test de restauration
Bloc **Test de restauration** : l'application relit la dernière archive, l'**extrait dans un dossier temporaire**, exige que le **fichier du monde (`Level.sav`) et `LevelMeta.sav` soient présents et non vides**, compte les joueurs, puis supprime tout. **Votre vrai monde n'est jamais touché.** Fréquence par défaut : tous les **7 jours** (réglable dans Application) ; un échec déclenche une alerte. Bouton **Tester maintenant**. Le dernier résultat est affiché (✔/✖, date, détail).

---

## 15. Diagnostic

Liste de contrôles de l'installation, chacun avec ✔/✖, un détail et, **s'il est en échec, un conseil en français** :

| Contrôle | Ce qu'il vérifie |
|---|---|
| PalServer.exe trouvé | Le fichier existe |
| SteamCMD trouvé | Le chemin est valide |
| Le dossier configuré est celui du serveur en cours d'exécution | Cohérence avec le processus réel |
| Dossier de sauvegarde du monde trouvé | `SaveGames` existe |
| Espace disque suffisant | Selon le seuil d'alerte (5 Go par défaut) |
| Dossier de sauvegarde accessible en écriture | Test d'écriture |
| Second emplacement accessible | Si configuré |
| Tous les mods activés sont présents | Cohérence de `PalModSettings.ini` |
| PalWorldSettings.ini lisible | Lecture du fichier |
| API REST activée / mot de passe défini / identique / port identique | Cohérence application ↔ serveur |
| API REST joignable | Le serveur répond (il doit être démarré) |

Boutons : **Relancer**. Deux blocs d'aide apparaissent selon le cas :
- **« Le serveur qui tourne n'est pas dans le dossier configuré »** → bouton **Utiliser le dossier du serveur en marche** (corrige le réglage en un clic).
- **« Corriger l'API REST en un clic »** → saisissez un mot de passe admin, **Appliquer** : active `RESTAPIEnabled`, fixe le port et le mot de passe (copie `.bak`), aligne l'application. Redémarrez ensuite le serveur.

---

## 16. Mobile et invités

### 16.1 Principe
L'application embarque une petite **page web** (« Mobile ») que vous ouvrez depuis un téléphone : état du serveur, joueurs, journal, graphiques, démarrer/arrêter/redémarrer, sauvegarde, annonce, expulsion. Elle est **désactivée par défaut**.

Condition : le PC doit être allumé, l'application ouverte (elle peut rester réduite).

### 16.2 Activer
Dans l'onglet **Mobile** : cochez **Activé** et définissez le **port** (8765 par défaut). L'application génère une **clé secrète** (128 bits). Bouton **Appliquer**. Au premier lancement, Windows demande d'autoriser l'application dans le pare-feu : cochez **Réseaux privés**.

Deux cartes d'adresse avec QR code : **« Chez vous (même Wi-Fi) »** et **« Partout (Tailscale) »**. Le QR code du propriétaire est **caché par défaut** (« Afficher le QR code » / « Le cacher ») car il contient la clé.

**Régénérer la clé** invalide l'ancienne immédiatement.

### 16.3 Accès hors de chez vous (4G) avec Tailscale
Étapes (Android) :
1. Installez **Tailscale** sur le PC et connectez-vous (compte Google, Microsoft ou Apple, gratuit).
2. Installez **Tailscale** sur le téléphone (Play Store), **même compte**, et activez la connexion (une icône de clé apparaît).
3. Dans l'application, cochez **Activé** : la carte « Partout (Tailscale) » apparaît.
4. Scannez son QR code avec l'appareil photo, ou copiez l'adresse. La page s'ouvre dans Chrome et se connecte seule. Menu ⋮ → « Ajouter à l'écran d'accueil » pour l'avoir comme une application.

Tout passe par le réseau privé chiffré de Tailscale : **aucune redirection de port** sur la box.

### 16.4 Pourquoi c'est sûr
- Chaque requête exige la clé (comparaison en temps constant, délai de 300 ms sur échec).
- Le serveur mobile **refuse les adresses publiques d'Internet**, même avec la bonne clé : seuls le réseau local, le lien local et **Tailscale** (100.64.0.0/10) sont acceptés.
- Réglages, configuration du monde, restauration, mise à jour et bannissement ne sont **jamais** exposés au mobile.

### 16.5 Invités : un accès limité pour vos amis
Bloc **Invités** : chaque invité reçoit **son propre QR code** avec uniquement les droits que vous choisissez.

Création (**Nouvelle invitation**) :
- **Nom** de l'invité.
- **Modèles** : *Spectateur* (état, joueurs, graphiques), *Modérateur* (+ journal, annonces, expulsion), *Contrôle total (sans réglages)* (+ sauvegarde, démarrer, redémarrer, arrêter).
- **Permissions** une par une (voir tableau ci-dessous ; les actions sensibles sont marquées ⚠).
- **Durée** : 1 heure, 24 heures, 7 jours, 30 jours ou illimitée.

**Révoquer** coupe un invité immédiatement, sans toucher aux autres. Le lien contient sa clé : traitez-le comme un mot de passe.

Permissions :

| Permission | Effet |
|---|---|
| Voir l'état du serveur | En ligne, CPU, RAM, nombre de joueurs |
| Voir les pseudos et l'historique | Noms des joueurs (masqués sans cette permission) |
| Voir le journal | Journal du serveur |
| Voir les graphiques | Historique |
| Envoyer une annonce | En jeu |
| Lancer une sauvegarde | Sauvegarde manuelle |
| Démarrer / Redémarrer / Arrêter ⚠ | Contrôle du serveur |
| Expulser un joueur ⚠ | Expulsion |

---

## 17. Application (réglages généraux)

Tous les réglages s'enregistrent avec le bouton **Enregistrer** en bas de la page. Le bouton **Tester les alertes** envoie une alerte de test (Discord/ntfy et notification Windows).

### 17.1 Serveur
- **Dossier du serveur (PalServer.exe)**, **Chemin de steamcmd.exe** (un dossier est accepté : l'application y cherche `steamcmd.exe`).
- **Arguments de lancement** (séparés par des espaces), par exemple `-useperfthreads`.
- **Mot de passe admin (API REST)** : doit être identique à celui du serveur (le Diagnostic le vérifie).
- **Afficher la console du serveur dans l'application** : lance le moteur directement dans une console virtuelle (§ 13.2).
- **Fichier journal du serveur** : chemin imposé (vide = détection automatique).

### 17.2 Démarrage de l'application
- **Lancer avec Windows (réduit dans la zone de notification)** : refusé dans une version de développement.
- **Démarrer le serveur automatiquement au lancement de l'application** (≈ 15 s après) : à combiner avec « Lancer avec Windows » pour un serveur qui revient seul après un redémarrage du PC.
- **Fermer la fenêtre = réduire dans la zone de notification**.
- **Redémarrage auto après crash**.

### 17.3 Mémoire
- **Redémarrer si RAM ≥ (%)** : redémarrage préventif si la mémoire dépasse ce seuil (vide = jamais), après 1 minute de préavis annoncé aux joueurs.
- **Mémoire : attendre que le serveur soit vide, au plus (minutes)** : plutôt que d'interrompre des joueurs, attend que le serveur soit vide (0 = redémarrer après 1 minute de préavis).
- **Annonces (minutes avant, ex. 15, 5, 1)** : paliers d'annonce avant un arrêt/redémarrage programmé.

### 17.4 Horaires programmés
Chaque ligne : **heure** (`HH:MM`, heure locale du PC), **action** (Démarrer / Redémarrer / Arrêter), **jours** (boutons L M M J V S D ; aucun filtre = tous les jours), et, pour Démarrer et Redémarrer, un **profil de configuration** à appliquer juste avant l'exécution. Interrupteur **Activés**.

- Avant un arrêt ou un redémarrage, les joueurs sont prévenus aux minutes indiquées (le palier le plus proche seulement), puis une sauvegarde est faite.
- Si l'application est fermée à l'heure prévue, l'action est **ignorée** (pas de rattrapage au-delà de 2 minutes).
- Deux règles à la même heure ne se gênent pas.
- Tout redémarrage programmé passe par le **cycle sûr** (§ 18.1).

### 17.5 Calendrier de saisons
Un **profil de configuration appliqué pendant une période** (« Semaine XP » du 1er au 7), puis retour automatique à la configuration d'avant.

Chaque événement : nom, date de début, date de fin (incluse), profil, annonce facultative. Le fonctionnement, application ouverte :
1. **À partir de 04:00** le premier jour, l'application enregistre la configuration actuelle sous un profil **`avant-<nom>`**, applique le profil de l'événement et redémarre le serveur proprement (préavis, sauvegarde, contrôle, retour arrière).
2. Le **lendemain de la fin**, elle réapplique `avant-<nom>` (retour à l'état d'avant) avec le même cycle sûr.
3. Si le serveur est arrêté à ce moment, le profil est appliqué **sans le démarrer**.

Créez d'abord le profil dans **Configuration**. Un message « Événement en cours » s'affiche quand un événement est actif. ⚠ non vérifié en réel.

### 17.6 Sauvegardes
**Backup automatique**, **Intervalle**, **Rétention par paliers**, **Sauvegarder aussi la fiche de chaque joueur**, **Test de restauration (jours)**, **Versions gardées par joueur**, **Nombre de backups conservés**, **Second emplacement**, **Dossier de destination** (voir chapitre 14).

### 17.7 Alertes et rapports
**Webhook Discord**, **URL ntfy**, **Notifications Windows**, **Résumé quotidien à (HH:MM)**, **Rapport hebdomadaire le (jour)**, **Seuil mémoire (%)**, **Alerte si aucune sauvegarde depuis (heures)** (6 par défaut), **Alerte si espace disque libre sous (Go)** (5 par défaut), cases **Crash / Connexion / Déconnexion**. Détails au chapitre 19.

### 17.8 Mise à jour automatique du serveur Palworld
Interrupteur **Activée**, **Vérifier toutes les (minutes, min. 15)**, **Préavis aux joueurs (minutes, max. 30)**, bouton **Vérifier maintenant**.

L'application compare le **numéro de build installé** (manifeste Steam du serveur) au **build public** de Steam (via SteamCMD). Si une nouvelle version existe :
1. alerte « Mise à jour… » ;
2. annonce aux joueurs présents (préavis) ;
3. **cycle sûr** : sauvegarde, arrêt, mise à jour SteamCMD, démarrage, contrôle, retour arrière si besoin.

**Une seule tentative par version** : en cas d'échec vous êtes alerté, sans boucle. La vérification tourne en arrière-plan.

### 17.9 Bot Discord
Voir le chapitre 20.

### 17.10 Auto-réparation
Interrupteur **Activée**, **Gel : redémarrer après (minutes)**, **Boucle de crashs : nombre** et **…en (minutes)**. Voir § 18.2 et 18.3.

### 17.11 Mods automatiques
**Mise à jour automatique**, **Vérifier toutes les (minutes, min. 30)**, liste des **Mods gérés**. Voir § 9.5.

### 17.12 Ouverture automatique du port de jeu (UPnP)
**Désactivée par défaut.** Si vous la cochez (après une confirmation), l'application demande à votre box d'ouvrir le **port UDP du jeu** (8211 par défaut ou `PublicPort`) vers ce PC, tant que le serveur tourne. Le bail est d'une heure, renouvelé toutes les 20 minutes ; la redirection est fermée si vous décochez l'option.

- Elle n'ouvre **jamais** l'API REST (8212) ni l'accès mobile (8765).
- Elle ne fonctionne que si **UPnP est activé sur la box**.
- **Elle expose votre serveur de jeu à Internet** : pensez au mot de passe serveur ou à la liste blanche.
- Bouton **Tester maintenant** : ouvre le port une fois et affiche l'adresse publique vue par la box.

⚠ non vérifié en réel.

### 17.13 Verrou par code
Définissez un **code** (4 à 32 caractères) et un délai de **reverrouillage** après inactivité (0 = jamais). Changer ou retirer le code exige l'ancien. Le code n'est jamais stocké en clair (condensat salé).

> C'est un **garde-fou** contre l'usage involontaire ou curieux, **pas** une protection contre quelqu'un qui a accès à votre session Windows ou à vos fichiers.

---

## 18. Les automatismes expliqués

### 18.1 Le cycle de redémarrage sûr
Utilisé par **tous** les redémarrages automatiques (horaires, mémoire, gel, mises à jour du serveur et des mods, saisons) et par le bouton **Mettre à jour**.

1. **Préavis** aux joueurs (si le serveur n'est pas vide), puis attente.
2. **Sauvegarde de sûreté** (l'arrêt propre en crée déjà une ; sinon l'application en fait une, `-avant-…`).
3. Arrêt propre, application du **profil** si demandé, **mise à jour SteamCMD** si demandée.
4. **Démarrage**.
5. **Contrôle pendant 6 minutes** : le processus doit rester vivant **et** l'API REST répondre (sans API configurée : processus vivant après 90 s).
6. Si le contrôle échoue — **retour arrière progressif** :
   1. arrêt, puis **désactivation des mods suspects** (ceux qui viennent de changer, ou tous si on ne sait pas) et nouveau démarrage ;
   2. si ça ne suffit pas : **restauration du monde** depuis la sauvegarde de sûreté et des réglages de mods d'origine, nouveau démarrage ;
   3. si rien ne marche : message « intervention nécessaire (voir l'onglet Journal) ».

Le résultat est envoyé aux alertes : ✅ réussi, ⚠️ réussi après désactivation de mods, ↩️ monde restauré, ❌ échec.

**Limite** : une mise à jour du **programme serveur** ne peut pas être annulée (SteamCMD ne revient pas en arrière) ; le retour arrière protège le **monde** et les **mods**.

Ces cycles tournent **en arrière-plan** : l'interface, les graphiques et la surveillance continuent pendant le préavis et le redémarrage. Un seul cycle à la fois.

### 18.2 Serveur gelé
Si le **processus tourne mais que l'API REST ne répond plus** depuis N minutes (5 par défaut) — **après avoir répondu au moins une fois** depuis le démarrage — l'application alerte et redémarre le serveur (cycle sûr). Garde-fous : jamais pendant les 4 premières minutes d'un démarrage, jamais si l'API est désactivée, et pas deux fois en moins de 15 minutes.

### 18.3 Crashs en boucle
Chaque crash est enregistré. Si le nombre atteint le seuil (3 en 10 minutes par défaut), la **relance automatique s'arrête** et vous êtes alerté, au lieu de boucler. Elle reprend quand vous **démarrez le serveur à la main**.

### 18.4 Arrêt non voulu
Si le serveur s'arrête hors de l'application (fenêtre fermée, crash), une sauvegarde `-arret-externe` est faite avant toute relance, et l'alerte de crash indique la **cause probable**. Un arrêt demandé depuis l'application ou les automatismes n'est jamais pris pour un crash.

### 18.5 Sauvegardes et santé
Toutes les 5 minutes, l'application contrôle : **aucune sauvegarde réussie depuis N heures** (serveur en marche depuis au moins N heures) et **espace disque faible**. Une alerte par type et par 6 heures au plus.

### 18.6 Ordre de priorité des événements
Pendant un cycle de maintenance, les alertes de crash et les redémarrages concurrents sont suspendus : la maintenance a la priorité.

---

## 19. Alertes et notifications

### 19.1 Canaux
- **Discord** (webhook), **ntfy** (notification sur le téléphone, via `https://ntfy.sh/<votre-sujet>`), **notifications Windows** (cases à cocher).
- Les trois canaux sont toujours **tous essayés** : une panne de Discord n'empêche pas ntfy.
- Un message Discord ne peut **jamais mentionner** personne (un joueur nommé `@everyone` ne notifie pas le salon).
- Anti-spam : un délai (5 min) entre deux alertes du même type (hors connexions).

### 19.2 Liste des alertes
| Événement | Message (extrait) |
|---|---|
| Crash | 🔴 Le serveur s'est arrêté de façon inattendue (+ cause probable) |
| Mémoire élevée | 🟠 Mémoire élevée : X % |
| Limite de RAM | 🟠 RAM du serveur : X Go (limite Y) |
| Connexion / déconnexion | 🟢 / ⚪ *nom* a rejoint / quitté |
| Sauvegarde échouée | ⚠️ Sauvegarde impossible |
| Sauvegarde trop ancienne | ⚠️ Aucune sauvegarde réussie depuis N h |
| Disque faible | ⚠️ Espace disque faible |
| Copie miroir en échec | ⚠️ Sauvegarde (copie miroir) impossible |
| Test de restauration échoué | ❌ Test de restauration échoué |
| Mise à jour du serveur | 🔄 Mise à jour… / résultat du cycle |
| Mods mis à jour | 🧩 Mods mis à jour… |
| Serveur gelé | 🧊 Le serveur ne répond plus… |
| Boucle de crashs | 🛑 N crashs en M min : relance arrêtée |
| UPnP | ⚠️ UPnP : … |
| Saison | 🎉 Début / 🏁 Fin de l'événement |

### 19.3 Résumé quotidien
À l'heure choisie (HH:MM) : observation de 24 h, joueurs différents et pic, plus longue session, CPU et mémoire moyens et maximum.

### 19.4 Rapport hebdomadaire
Le jour choisi, à l'heure du résumé quotidien (20:00 par défaut) : connexions et joueurs différents, **pic** de joueurs en même temps, **disponibilité observée**, **joueurs les plus actifs** (temps de jeu), **plus grosses progressions** de niveau, **nouveaux joueurs**. Envoyé **une fois par semaine** ; si l'application était fermée à l'heure prévue, il part à sa réouverture dans la semaine.

### 19.5 Tester
**Application → Tester les alertes**.

---

## 20. Bot Discord

Le bot répond à des **commandes slash** dans Discord. Il se connecte **depuis votre PC vers Discord** (aucun port ouvert chez vous).

### 20.1 Mise en place pas à pas
1. Allez sur `discord.com/developers`, **créez une application**, ajoutez un **Bot**.
2. Copiez le **jeton du bot** (« Reset Token » puis copier) ; gardez-le secret.
3. Invitez le bot sur votre serveur Discord (page *OAuth2 → URL Generator*, portée **`applications.commands`**).
4. Dans Discord : Paramètres → Avancés → **Mode développeur**, puis clic droit sur votre profil → **Copier l'identifiant**.
5. Dans l'application (**Application → Bot Discord**) : collez le **jeton**, collez votre **identifiant** dans **Identifiants Discord autorisés** (plusieurs, séparés par des virgules), cochez **Activé**, **Enregistrer**.
6. Les commandes apparaissent dans Discord (quelques minutes au plus).

### 20.2 Commandes
| Commande | Effet | Nécessite « contrôle » |
|---|---|---|
| `/statut` | État, joueurs, FPS, jour, durée de fonctionnement, CPU, RAM | Non |
| `/joueurs` | Joueurs connectés avec leur niveau | Non |
| `/sauvegarde` | Crée une sauvegarde | Oui |
| `/demarrer` | Démarre le serveur | Oui |
| `/arreter` | Arrête le serveur (avec sauvegarde) | Oui |
| `/redemarrer` | Redémarre le serveur (avec sauvegarde) | Oui |
| `/annonce message` | Envoie une annonce en jeu (200 caractères) | Oui |

### 20.3 Sécurité du bot
- **Seuls les identifiants listés** peuvent l'utiliser ; **une liste vide = personne**.
- Case **« Autoriser le contrôle »** décochée par défaut : le bot est alors en **lecture seule**.
- Les réponses sont **éphémères** (visibles seulement par la personne qui a tapé la commande).
- Chaque commande est écrite dans le **journal d'audit** (`Discord · identifiant`).
- Le jeton est stocké dans le **Gestionnaire d'identifiants Windows**.
- Les actions longues (redémarrage) utilisent une réponse différée (Discord impose 3 secondes).
- Reconnexion automatique avec attente croissante (5 minutes au maximum) ; un jeton refusé ne provoque pas de rafale de tentatives.

⚠ non vérifié contre le vrai Discord.

---

## 21. Sécurité et vie privée

### 21.1 Ce qui est exposé ou non
| Élément | Exposition |
|---|---|
| API REST du serveur (8212) | **Local uniquement.** Ne la redirigez jamais. |
| Page Mobile (8765) | Réseau local et Tailscale **uniquement** ; adresses publiques refusées |
| Port du jeu (8211 UDP) | Public si vous le redirigez ou activez l'UPnP |
| Bot Discord | Connexion **sortante** uniquement |
| Application ↔ Internet | Releases GitHub (mises à jour), `api.ipify.org` (au clic), Steam (SteamCMD), Discord/ntfy si configurés, steamcmd.zip de Valve (installation) |

### 21.2 Secrets
Mot de passe admin, jetons (mobile, invités, bot Discord), webhooks : stockés dans le **Gestionnaire d'identifiants Windows**, remplacés par `@secret` dans `settings.json`. La migration depuis un ancien fichier en clair est automatique. Si le magasin est indisponible, la valeur reste en clair plutôt que d'être perdue. Conséquence : une copie de `settings.json` ne contient plus les secrets (ils sont liés à votre compte Windows).

### 21.3 Ce que l'application ne fait pas
- Elle **ne lit ni ne conserve les adresses IP** des joueurs.
- Elle ne télécharge rien d'autre que ce qui est listé ci-dessus.
- Elle ne signe pas son installeur (SmartScreen avertit).

### 21.4 Bonnes pratiques
1. Mot de passe serveur ou **liste blanche** si le port de jeu est public.
2. Donnez aux invités le **minimum** de droits, avec une **durée**.
3. Ne partagez jamais un lien/QR code sans y penser : il contient la clé.
4. Gardez le bot Discord en **lecture seule** si vous n'avez pas besoin du contrôle.
5. Activez le **test de restauration** et le **second emplacement** de sauvegarde.

---

## 22. Où sont rangés les fichiers

### 22.1 Données de l'application — `%APPDATA%\dev.palmanager.app\`

| Fichier / dossier | Contenu |
|---|---|
| `settings.json` | Réglages (secrets remplacés par `@secret`) ; copie `.illisible-…` si le fichier a été corrompu |
| `players.json` | Carnet des joueurs |
| `bans.json` | Bannissements faits depuis l'application |
| `history\` | Échantillons (7 jours) et événements (30 jours) |
| `profiles\` | Profils de configuration (`*.ini`) |
| `audit.jsonl` | Journal d'audit (90 jours) |
| `season-state.json` | Événement de saison appliqué |
| `restore-test.json` | Dernier test de restauration |
| `server-console.log` (+ `.old`) | Console du serveur lancé par l'application |
| `backups\` | Sauvegardes (destination par défaut) |
| `backups\joueurs\<ID>\` | Sauvegardes individuelles des joueurs |

### 22.2 Dans le dossier du serveur
| Chemin | Contenu |
|---|---|
| `Pal\Saved\Config\WindowsServer\PalWorldSettings.ini` | Configuration (+ `.ini.bak` créées par l'application) |
| `Pal\Saved\SaveGames\0\<monde>\` | Le monde (`Level.sav`, `LevelMeta.sav`, `Players\`) |
| `Pal\Saved\SaveGames.bak\` | Monde précédent, après une restauration |
| `Mods\PalModSettings.ini` | Liste des mods actifs (+ `.bak`) |
| `steamapps\workshop\content\1623730\<id>\` | Mods téléchargés |
| `steamapps\appmanifest_2394010.acf` | Build installé du serveur |

### 22.3 Nom d'un fichier de sauvegarde
`palworld-AAAAMMJJ-HHMMSS-<origine>.zip`, par exemple `palworld-20261001-040000-auto.zip`. Pour une sauvegarde joueur : `AAAAMMJJ-HHMMSSmmm-<empreinte>.zip` dans le dossier du joueur.

---

## 23. Recettes pratiques

### 23.1 Configurer un serveur « qui se gère seul »
1. Chapitre 3 : installez/choisissez le serveur, définissez le mot de passe admin.
2. **Application** : cochez *Lancer avec Windows*, *Démarrer le serveur automatiquement*, *Redémarrage auto après crash*, *Fermer la fenêtre = réduire*.
3. **Horaires** : un redémarrage quotidien à 04:00 (préavis 15, 5, 1 min).
4. **Sauvegardes** : intervalle 30 min, sauvegarde à l'arrêt, **second emplacement**, test de restauration.
5. **Mise à jour automatique** du serveur et des mods : activées.
6. **Alertes** : Discord ou ntfy, test avec « Tester les alertes ».
7. **Auto-réparation** : activée (valeurs par défaut).

### 23.2 Organiser une « semaine XP ×2 »
1. **Configuration** : réglez le taux d'XP, puis **Profils → Enregistrer sous ce nom** (« XP2 »). *(Enregistrez d'abord la configuration sur le disque.)*
2. **Application → Calendrier de saisons** : ajoutez l'événement (dates, profil « XP2 », annonce).
3. Laissez l'application ouverte : le premier jour à partir de 04:00 elle applique, redémarre et annonce ; le lendemain de la fin, elle revient à l'état d'avant.

### 23.3 Avant un gros événement (boss, raid)
**Sauvegardes → Sauvegarde protégée** (nom : `avant-raid`). Elle ne sera jamais supprimée. En cas de catastrophe : **Revenir à celle-ci**.

### 23.4 Installer des mods en sécurité
1. Activez les **mises à jour automatiques** des mods.
2. **Sauvegarde protégée** avant d'ajouter.
3. **Mods → Ajouter un mod**, puis redémarrez le serveur.
4. Si le serveur ne repart pas, le cycle sûr désactive les mods fautifs automatiquement (§ 18.1). Réactivez-les un par un pour trouver le coupable.
5. Enregistrez un **pack « Avec mods »** et un pack « Vanilla » pour basculer facilement.

### 23.5 Restaurer le monde après un problème
1. **Sauvegardes** : repérez la bonne archive (date, origine).
2. **Vérifier** (facultatif), puis **Revenir à celle-ci**.
3. Le serveur redémarre sur l'ancien monde ; l'état d'avant reste disponible (`-avant-restauration`).

### 23.6 Récupérer un joueur dont la fiche est corrompue
Joueurs → Historique → **Détails** → choisissez une sauvegarde de la fiche → **Restaurer cette fiche**. Si ses objets ou ses Pals manquent, c'est dans `Level.sav` : utilisez une sauvegarde du **monde entier**.

### 23.7 Surveiller depuis le téléphone
Chapitre 16 : activez Mobile, installez Tailscale sur le PC et le téléphone, scannez la carte « Partout (Tailscale) », ajoutez la page à l'écran d'accueil.

### 23.8 Donner un accès limité à un ami modérateur
**Mobile → Nouvelle invitation** : nom, modèle *Modérateur*, durée 7 jours. Envoyez-lui son QR code. Révoquez-le dès que vous voulez.

### 23.9 Piloter par Discord
Chapitre 20 : bot en **lecture seule** pour tout le monde, contrôle activé seulement pour vous.

### 23.10 Savoir ce qui s'est passé
**Journal → Journal d'audit** (qui a fait quoi) et **Analyse du journal** (cause probable d'un crash).

---

## 24. Limites connues et points non vérifiés

### 24.1 Fonctions non vérifiées sur un vrai serveur
Tout ce qui suit est couvert par des tests de logique, mais **pas éprouvé en conditions réelles** :
- capture de la console par console virtuelle (ConPTY) et démarrage direct du moteur ;
- priorité et cœurs du processus ;
- cycle de retour arrière complet (désactivation des mods, restauration) ;
- détection du build Steam par SteamCMD (le format de sortie peut différer) ;
- téléchargement et mise à jour des mods (le Workshop refuse parfois le téléchargement anonyme) ;
- UPnP, bot Discord, calendrier de saisons, rapport hebdomadaire, test de restauration sur un vrai monde ;
- niveaux, positions et constructions des joueurs (dépend de ce que l'API de votre version renvoie) ;
- politique de sécurité de la fenêtre (si l'interface s'affiche blanche, signalez-le).

### 24.2 Limites de conception
- **Un seul serveur** est géré.
- **Fiche joueur ≠ joueur complet** : objets, Pals et bases sont dans `Level.sav` ; **pas de transfert d'un joueur** vers un autre serveur.
- **Application fermée = plus de surveillance** ni d'automatismes.
- Un serveur lancé **hors de l'application** est « adopté » (surveillé, arrêtable) mais sa **console ne peut pas être captée**.
- La mise à jour du **binaire** du serveur ne peut pas être annulée.
- **Le verrou par code** est un garde-fou, pas une sécurité.
- **Traduction anglaise** partielle ; **pas de signature** de l'installeur.
- L'historique n'est purgé qu'au **lancement** de l'application.

---

## 25. Dépannage

### 25.1 « API REST joignable » est rouge
1. Le serveur est-il **démarré** ?
2. Diagnostic : le mot de passe application = serveur ? l'API est-elle activée ? Utilisez **Corriger l'API REST en un clic**, puis redémarrez le serveur.
3. Pare-feu Windows / port REST différent de celui du serveur (le Diagnostic le signale).

### 25.2 Le journal du tableau de bord est vide
- Avez-vous démarré le serveur **depuis l'application** ? Sinon arrêtez-le et relancez-le avec **Démarrer** (§ 13.2).
- Le texte sous le filtre indique le fichier lu ou ce qui a été cherché. Vous pouvez imposer un fichier (Application).
- Une fenêtre noire apparaît quand même ? L'application s'est repliée sur l'ancien lancement : décochez l'option « Afficher la console du serveur dans l'application ».

### 25.3 Les sauvegardes échouent
- Diagnostic : « Dossier de sauvegarde du monde trouvé » (le dossier du serveur est-il le bon ?), « accessible en écriture », espace disque.
- « le dossier configuré n'est pas celui du serveur en marche » → bouton **Utiliser le dossier du serveur en marche**.

### 25.4 Le serveur ne démarre pas
- Journal → Analyse (cause probable).
- Un mod en cause ? Mods → désactivez-les, ou appliquez un pack « Vanilla ».
- Configuration modifiée ? Restaurez la copie `PalWorldSettings.ini.bak`.

### 25.5 L'application s'ouvre blanche / un élément manque
Signalez-le (c'est probablement la politique de sécurité de la fenêtre). Une réinstallation de la version précédente règle le problème.

### 25.6 Le serveur redémarre en boucle
Auto-réparation : la relance s'arrête après 3 crashs en 10 minutes et vous alerte. Lisez le Journal, corrigez, puis démarrez **à la main**.

### 25.7 Les automatismes ne se déclenchent pas
L'application doit **tourner** (elle peut être réduite). Vérifiez l'interrupteur **Activés** des horaires, et que le serveur est bien surveillé (Tableau de bord).

### 25.8 Le téléphone n'accède pas à la page
- Mobile activé et **Appliquer** cliqué ? Pare-feu : « Réseaux privés » coché ?
- Hors de chez vous : Tailscale connecté **sur les deux appareils**, même compte (ou partage accepté).
- La clé a-t-elle été régénérée ? Rescannez le QR code.

### 25.9 « Mettre à jour » échoue
- Diagnostic : SteamCMD trouvé ?
- Le message du cycle indique l'étape ; le serveur est relancé tel quel en cas d'échec de SteamCMD.

### 25.10 J'ai perdu mes réglages
`settings.json` illisible : l'application démarre avec les réglages par défaut et garde une copie `settings.json.illisible-<date>` dans le même dossier : vous pouvez la récupérer. Les secrets restent dans le Gestionnaire d'identifiants.

### 25.11 Le code du verrou est oublié
Quittez l'application, supprimez les champs `lock` de `settings.json` (ou le fichier entier, vous perdrez alors vos réglages), relancez. (Ce verrou n'est pas une sécurité.)

### 25.12 Quelle information donner pour un signalement
Version de l'application (bandeau/installeur), l'écran concerné (capture), le texte exact du message d'erreur, et — si possible — la sortie de **Diagnostic** et de **Journal → Analyse**.

---

## 26. Glossaire

| Terme | Signification |
|---|---|
| **API REST** | Interface de service du serveur Palworld (port 8212) utilisée par l'application |
| **AutoSaveSpan** | Intervalle (en secondes) auquel le serveur écrit lui-même le monde |
| **Build** | Numéro de version Steam du serveur |
| **ConPTY** | Console virtuelle de Windows, qui permet de lire la sortie d'un programme comme dans une vraie fenêtre |
| **Cycle sûr** | Redémarrage automatique avec sauvegarde, contrôle et retour arrière |
| **Fiche joueur** | Fichiers `Players\<id>.sav` : niveau, statistiques, technologies |
| **Level.sav** | Fichier principal du monde : objets, Pals, bases |
| **Liste blanche** | Liste des joueurs seuls autorisés à entrer |
| **Miroir** | Seconde copie de chaque sauvegarde, dans un autre dossier |
| **Profil** | Ensemble enregistré de réglages du monde |
| **Pack de mods** | Ensemble enregistré de mods actifs |
| **Rotation** | Suppression automatique des plus anciennes sauvegardes |
| **SteamCMD** | Outil en ligne de commande de Valve (installation, mises à jour, mods) |
| **Tailscale** | Réseau privé chiffré qui relie vos appareils sans ouvrir de port |
| **UPnP** | Protocole par lequel un programme demande à la box d'ouvrir un port |
| **Webhook** | Adresse qui permet à un programme de poster un message dans un salon Discord |
| **ntfy** | Service gratuit de notifications sur téléphone |
| **Workshop** | Boutique de mods de Steam |

---

## 27. Annexes (tableaux de référence)

### 27.1 Ports

| Port | Protocole | Usage | À exposer ? |
|---|---|---|---|
| 8211 | UDP | Jeu | Oui, si des amis entrent depuis Internet (redirection ou UPnP) |
| 8212 | TCP | API REST | **Jamais** |
| 8765 | TCP | Page Mobile | **Jamais** vers Internet (réseau local / Tailscale) |

### 27.2 Commandes Discord
Voir le § 20.2.

### 27.3 Valeurs par défaut importantes

| Réglage | Défaut |
|---|---|
| Sauvegarde automatique | activée, toutes les 30 min, conservées : 10 |
| Sauvegarde à l'arrêt | activée |
| Sauvegarde des fiches joueurs | activée, 10 versions |
| Test de restauration | tous les 7 jours |
| Alerte « pas de sauvegarde » | 6 h |
| Alerte « disque faible » | 5 Go |
| Mise à jour auto du serveur | désactivée (vérifs 60 min, préavis 5 min) |
| Mods : mise à jour auto | désactivée (vérifs 120 min) |
| Auto-réparation | activée (gel 5 min, boucle : 3 crashs / 10 min) |
| UPnP | désactivé |
| Bot Discord | désactivé, lecture seule |
| Accès mobile | désactivé, port 8765 |
| Annonces | désactivées |
| Redémarrage auto après crash | activé |
| Fermer = réduire | désactivé |

### 27.4 Raccourcis utiles
- **Clic droit sur l'icône** (zone de notification) → *Ouvrir* / *Quitter*.
- **Filtre du journal** : saisissez `error`, `joined`, un pseudo…
- **Menu ⋮ de Chrome → Ajouter à l'écran d'accueil** : la page Mobile devient une application.

### 27.5 Tâches planifiées de l'application (récapitulatif)

| Fréquence | Tâche |
|---|---|
| 5 secondes | Mesure CPU/RAM/joueurs, alertes, horaires, liste blanche, rappels |
| 30 secondes | Échantillon d'historique |
| 5 minutes | Santé des sauvegardes, calendrier de saisons |
| 10 minutes | Vérification de l'échéance du test de restauration |
| 20 minutes | Renouvellement de la redirection UPnP |
| 6 heures | Recherche d'une mise à jour de l'application |
| Réglable | Sauvegardes, vérification de mise à jour du serveur (≥ 15 min) et des mods (≥ 30 min) |

---

*Fin du manuel. Pour l'architecture technique, voir `docs/ARCHITECTURE.md` ; pour l'évolution prévue, `docs/ROADMAP.md`.*
