// Schéma d'édition de PalWorldSettings.ini (clés issues de la doc Palworld ; à compléter au fil des
// versions). Toute clé absente d'ici reste éditable dans l'onglet « Avancé » et préservée à l'écriture.
export type Field =
  | { key: string; label: string; type: "text" | "password"; group: string }
  | { key: string; label: string; type: "number"; group: string; min?: number; max?: number; step?: number }
  | { key: string; label: string; type: "bool"; group: string }
  | { key: string; label: string; type: "enum"; group: string; options: string[] };

const t = (key: string, label: string, group: string, password = false): Field => ({ key, label, type: password ? "password" : "text", group });
const n = (key: string, label: string, group: string, min = 0, max = 10, step = 0.1): Field => ({ key, label, type: "number", group, min, max, step });
const b = (key: string, label: string, group: string): Field => ({ key, label, type: "bool", group });
const e = (key: string, label: string, group: string, options: string[]): Field => ({ key, label, type: "enum", group, options });

export const SCHEMA: Field[] = [
  // Général
  t("ServerName", "Nom du serveur", "Général"), t("ServerDescription", "Description", "Général"),
  t("ServerPassword", "Mot de passe", "Général", true), t("AdminPassword", "Mot de passe admin", "Général", true),
  n("ServerPlayerMaxNum", "Joueurs max", "Général", 1, 32, 1), n("CoopPlayerMaxNum", "Joueurs max en coop", "Général", 1, 4, 1),
  t("Region", "Région", "Général"), b("bShowPlayerList", "Afficher la liste des joueurs", "Général"),
  b("bIsShowJoinLeftMessage", "Messages de connexion/départ", "Général"), n("ChatPostLimitPerMinute", "Limite de messages/min", "Général", 1, 100, 1),
  // Réseau & API
  n("PublicPort", "Port du jeu", "Réseau", 1, 65535, 1), t("PublicIP", "IP publique", "Réseau"),
  b("RESTAPIEnabled", "API REST activée", "Réseau"), n("RESTAPIPort", "Port API REST", "Réseau", 1, 65535, 1),
  b("RCONEnabled", "RCON activé", "Réseau"), n("RCONPort", "Port RCON", "Réseau", 1, 65535, 1),
  b("bUseAuth", "Authentification", "Réseau"), t("BanListURL", "URL de la liste de bannis", "Réseau"),
  // Taux
  n("ExpRate", "Multiplicateur d'XP", "Taux", 0.1, 20), n("PalCaptureRate", "Taux de capture", "Taux", 0.5, 2),
  n("PalSpawnNumRate", "Apparition des Pals", "Taux", 0.5, 3), n("DayTimeSpeedRate", "Vitesse du jour", "Taux", 0.1, 5),
  n("NightTimeSpeedRate", "Vitesse de la nuit", "Taux", 0.1, 5), n("WorkSpeedRate", "Vitesse de travail", "Taux", 0.1, 10),
  n("PalEggDefaultHatchingTime", "Temps d'éclosion (h)", "Taux", 0, 240, 1), n("ItemWeightRate", "Poids des objets", "Taux", 0, 10),
  n("CollectionDropRate", "Butin de récolte", "Taux", 0.5, 3), n("EnemyDropItemRate", "Butin des ennemis", "Taux", 0.5, 3),
  n("CollectionObjectHpRate", "PV des ressources", "Taux", 0.5, 3), n("CollectionObjectRespawnSpeedRate", "Repousse des ressources", "Taux", 0.5, 3),
  // Combat
  b("bIsPvP", "PvP", "Combat"), b("bEnablePlayerToPlayerDamage", "Dégâts joueur contre joueur", "Combat"),
  b("bEnableFriendlyFire", "Tirs alliés", "Combat"), b("bEnableInvaderEnemy", "Raids ennemis", "Combat"),
  n("PalDamageRateAttack", "Dégâts infligés par les Pals", "Combat", 0.1, 5), n("PalDamageRateDefense", "Dégâts subis par les Pals", "Combat", 0.1, 5),
  n("PlayerDamageRateAttack", "Dégâts infligés par le joueur", "Combat", 0.1, 5), n("PlayerDamageRateDefense", "Dégâts subis par le joueur", "Combat", 0.1, 5),
  b("bEnableAimAssistPad", "Aide à la visée (manette)", "Combat"), b("bEnableAimAssistKeyboard", "Aide à la visée (clavier)", "Combat"),
  b("EnablePredatorBossPal", "Pals prédateurs boss", "Combat"),
  // Survie
  e("DeathPenalty", "Pénalité de mort", "Survie", ["None", "Item", "ItemAndEquipment", "All"]),
  b("bHardcore", "Mode hardcore", "Survie"), b("bPalLost", "Perte des Pals à la mort", "Survie"),
  n("PlayerStomachDecreaceRate", "Faim du joueur", "Survie", 0.1, 5), n("PlayerStaminaDecreaceRate", "Endurance du joueur", "Survie", 0.1, 5),
  n("PlayerAutoHPRegeneRate", "Régénération PV joueur", "Survie", 0.1, 5), n("PlayerAutoHpRegeneRateInSleep", "Régénération PV joueur (sommeil)", "Survie", 0.1, 5),
  n("PalStomachDecreaceRate", "Faim des Pals", "Survie", 0.1, 5), n("PalStaminaDecreaceRate", "Endurance des Pals", "Survie", 0.1, 5),
  n("PalAutoHPRegeneRate", "Régénération PV Pals", "Survie", 0.1, 5), n("PalAutoHpRegeneRateInSleep", "Régénération PV Pals (sommeil)", "Survie", 0.1, 5),
  b("bEnableFastTravel", "Voyage rapide", "Survie"), b("bExistPlayerAfterLogout", "Le joueur reste après déconnexion", "Survie"),
  // Bases & guildes
  n("BaseCampMaxNum", "Bases max (serveur)", "Bases & guildes", 1, 1000, 1), n("BaseCampWorkerMaxNum", "Travailleurs max par base", "Bases & guildes", 1, 50, 1),
  n("BaseCampMaxNumInGuild", "Bases max par guilde", "Bases & guildes", 1, 10, 1), n("GuildPlayerMaxNum", "Joueurs max par guilde", "Bases & guildes", 1, 100, 1),
  b("bAutoResetGuildNoOnlinePlayers", "Réinitialiser les guildes inactives", "Bases & guildes"),
  n("AutoResetGuildTimeNoOnlinePlayers", "Délai d'inactivité guilde (h)", "Bases & guildes", 1, 1000, 1),
  n("BuildObjectHpRate", "PV des constructions", "Bases & guildes", 0.5, 3), n("BuildObjectDamageRate", "Dégâts aux constructions", "Bases & guildes", 0.5, 3),
  n("BuildObjectDeteriorationDamageRate", "Dégradation des constructions", "Bases & guildes", 0, 10), b("bBuildAreaLimit", "Limiter la construction près des structures", "Bases & guildes"),
  n("MaxBuildingLimitNum", "Limite de constructions (0 = illimité)", "Bases & guildes", 0, 100000, 1),
  b("bEnableDefenseOtherGuildPlayer", "Défendre contre les autres guildes", "Bases & guildes"),
  // Système
  n("AutoSaveSpan", "Sauvegarde auto (s)", "Système", 30, 3600, 1), b("bIsUseBackupSaveData", "Sauvegardes internes du jeu", "Système"),
  n("DropItemMaxNum", "Objets au sol max", "Système", 100, 10000, 1), n("DropItemAliveMaxHours", "Durée de vie des objets au sol (h)", "Système", 0.1, 24),
  n("SupplyDropSpan", "Intervalle des largages (min)", "Système", 1, 1440, 1), n("ServerReplicatePawnCullDistance", "Distance de réplication des Pals", "Système", 5000, 15000, 100),
  e("Difficulty", "Difficulté", "Système", ["None", "Normal", "Difficult"]), e("LogFormatType", "Format des logs", "Système", ["Text", "Json"]),
];
export const GROUPS = [...new Set(SCHEMA.map((f) => f.group))];

export interface Preset { name: string; description: string; values: Record<string, string> }
// Préréglages : ne touchent que les clés listées. Valeurs de départ raisonnables, à ajuster.
export const PRESETS: Preset[] = [
  { name: "Décontracté", description: "Progression rapide, peu de contraintes", values: {
    ExpRate: "2.000000", PalCaptureRate: "1.500000", DeathPenalty: "None", PlayerStomachDecreaceRate: "0.500000",
    PalStomachDecreaceRate: "0.500000", WorkSpeedRate: "2.000000", bEnableInvaderEnemy: "False", PalEggDefaultHatchingTime: "1.000000" } },
  { name: "Standard", description: "Équilibre proche du jeu de base", values: {
    ExpRate: "1.000000", PalCaptureRate: "1.000000", DeathPenalty: "All", PlayerStomachDecreaceRate: "1.000000",
    PalStomachDecreaceRate: "1.000000", WorkSpeedRate: "1.000000", bEnableInvaderEnemy: "True", PalEggDefaultHatchingTime: "72.000000" } },
  { name: "Difficile", description: "Plus de risques, moins de ressources", values: {
    ExpRate: "0.800000", PalCaptureRate: "0.800000", DeathPenalty: "ItemAndEquipment", PlayerStomachDecreaceRate: "1.500000",
    PalStomachDecreaceRate: "1.500000", PlayerDamageRateDefense: "1.500000", bEnableInvaderEnemy: "True", bHardcore: "False" } },
];
