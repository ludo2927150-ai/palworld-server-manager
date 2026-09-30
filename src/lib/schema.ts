// Schéma d'édition de PalWorldSettings.ini : libellé, type, bornes. Les clés absentes du schéma
// restent éditables dans l'onglet « Avancé » (texte brut) et sont préservées à l'écriture.
export type Field =
  | { key: string; label: string; type: "text" | "password"; group: string }
  | { key: string; label: string; type: "number"; group: string; min?: number; max?: number; step?: number }
  | { key: string; label: string; type: "bool"; group: string }
  | { key: string; label: string; type: "enum"; group: string; options: string[] };

export const SCHEMA: Field[] = [
  { key: "ServerName", label: "Nom du serveur", type: "text", group: "Général" },
  { key: "ServerDescription", label: "Description", type: "text", group: "Général" },
  { key: "ServerPassword", label: "Mot de passe", type: "password", group: "Général" },
  { key: "AdminPassword", label: "Mot de passe admin", type: "password", group: "Général" },
  { key: "ServerPlayerMaxNum", label: "Joueurs max", type: "number", group: "Général", min: 1, max: 32 },
  { key: "PublicPort", label: "Port", type: "number", group: "Général", min: 1, max: 65535 },
  { key: "RESTAPIEnabled", label: "API REST activée", type: "bool", group: "Général" },
  { key: "RESTAPIPort", label: "Port API REST", type: "number", group: "Général", min: 1, max: 65535 },
  { key: "ExpRate", label: "Multiplicateur d'XP", type: "number", group: "Taux", min: 0.1, max: 20, step: 0.1 },
  { key: "PalCaptureRate", label: "Taux de capture", type: "number", group: "Taux", min: 0.5, max: 2, step: 0.1 },
  { key: "PalSpawnNumRate", label: "Apparition des Pals", type: "number", group: "Taux", min: 0.5, max: 3, step: 0.1 },
  { key: "DayTimeSpeedRate", label: "Vitesse du jour", type: "number", group: "Taux", min: 0.1, max: 5, step: 0.1 },
  { key: "NightTimeSpeedRate", label: "Vitesse de la nuit", type: "number", group: "Taux", min: 0.1, max: 5, step: 0.1 },
  { key: "bIsPvP", label: "PvP", type: "bool", group: "Règles" },
  { key: "bEnableInvaderEnemy", label: "Raids ennemis", type: "bool", group: "Règles" },
  { key: "DeathPenalty", label: "Pénalité de mort", type: "enum", group: "Règles", options: ["None", "Item", "ItemAndEquipment", "All"] },
  { key: "Difficulty", label: "Difficulté", type: "enum", group: "Règles", options: ["None", "Normal", "Difficult"] },
  { key: "AutoSaveSpan", label: "Sauvegarde auto (s)", type: "number", group: "Règles", min: 30, max: 3600 },
];
export const GROUPS = [...new Set(SCHEMA.map((f) => f.group))];
