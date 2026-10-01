// Traduction minimale : le texte français sert de clé ; sans entrée anglaise on affiche le français.
export type Lang = "fr" | "en";

const EN: Record<string, string> = {
  "Tableau de bord": "Dashboard", "Rejoindre": "Join", "Historique": "History", "Performance": "Performance", "Mods": "Mods",
  "Joueurs": "Players", "Annonces": "Announcements", "Configuration": "Settings", "Journal": "Log", "Sauvegardes": "Backups",
  "Diagnostic": "Diagnostics", "Mobile": "Mobile", "Application": "App",
  "MODE DÉMO — données fictives affichées dans le navigateur. Lancez « npm run tauri dev » (ou l'installeur) pour piloter un vrai serveur.":
    "DEMO MODE — fake data shown in the browser. Run “npm run tauri dev” (or the installer) to manage a real server.",
  "Nouvelle version disponible : {latest} (vous avez {current}).": "New version available: {latest} (you have {current}).",
  "Télécharger et installer": "Download and install", "Téléchargement…": "Downloading…", "Plus tard": "Later", "Langue": "Language",
  // Assistant
  "Bienvenue — configuration en 5 étapes": "Welcome — 5-step setup", "Passer": "Skip", "Précédent": "Back", "Suivant": "Next", "Terminer": "Finish",
  "Dossier du serveur": "Server folder", "SteamCMD": "SteamCMD", "API REST": "REST API", "Options": "Options", "Vérification": "Check",
  "Appliquer": "Apply", "Lancer le diagnostic": "Run diagnostics", "Mot de passe admin": "Admin password",
  "Aucune installation détectée automatiquement. Saisissez le chemin, ou installez le serveur via SteamCMD (onglet Application).":
    "No installation detected automatically. Enter the path, or install the server through SteamCMD (App tab).",
  "Relancer le serveur après un crash": "Restart the server after a crash",
  "Démarrer le serveur au lancement de l'application": "Start the server when the app launches",
  "Fermer la fenêtre = réduire dans la zone de notification": "Closing the window = minimize to the system tray",
  // Joueurs
  "En ligne": "Online", "Bannis": "Banned", "Liste blanche": "Whitelist", "Expulser": "Kick", "Bannir": "Ban", "Débannir": "Unban",
  "Autoriser": "Allow", "Retirer": "Remove", "Ajouter": "Add", "Envoyer": "Send", "Annonce à tous les joueurs": "Announcement to all players",
  "Personne en ligne.": "Nobody online.", "Niveau": "Level", "Nom": "Name", "banni": "banned", "autorisé": "allowed",
  "Dernière vue": "Last seen", "Temps de jeu": "Play time", "Première vue": "First seen", "maintenant": "now",
  "Activée (les autres sont expulsés)": "Enabled (others are kicked)", "Aucun joueur autorisé.": "No allowed players.",
  "Message montré à l'expulsé": "Message shown to the kicked player",
  "La liste est vide : par sécurité, personne n'est expulsé tant que vous n'avez ajouté aucun joueur.": "The list is empty: for safety, nobody is kicked until you add at least one player.",
};

let lang: Lang = (() => { try { return localStorage.getItem("lang") === "en" ? "en" : "fr"; } catch { return "fr"; } })();
export const getLang = () => lang;
export function setLang(l: Lang) { lang = l; try { localStorage.setItem("lang", l); } catch { /* ignoré */ } }

export function t(fr: string, vars?: Record<string, string | number>): string {
  let s = lang === "en" ? EN[fr] ?? fr : fr;
  if (vars) for (const [k, v] of Object.entries(vars)) s = s.split(`{${k}}`).join(String(v));
  return s;
}
