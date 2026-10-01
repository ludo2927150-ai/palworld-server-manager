// Pont IPC. Hors Tauri (npm run dev dans un navigateur) on bascule sur des données factices.
import type { AppSettings, BackupInfo, Check, BanEntry, VerifyReport, ProfileInfo, ServerUpdateInfo, UpdateInfo, KnownPlayer, PlayerSnapshot, AuditEntry, LockStatus, RestoreTestView, SeasonState, Finding, Guest, ModsState, LogChunk, Opt, Perm, RemoteInfo, Sample, Session, Snapshot, SystemInfo, WorldSettings } from "./types";
import { mock } from "./mock";

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function callRaw<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) return mock<T>(cmd, args);
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

// ───────── Indicateur de chargement sur les boutons ─────────
// Un bouton cliqué qui déclenche une commande affiche un petit rond tournant (attribut `data-loading`, voir index.css) jusqu'à la
// fin de la tâche, succès ou échec. Pendant ce temps il est insensible aux clics (pas de double démarrage). Les commandes de
// simple lecture périodique (état, journal…) n'y sont jamais associées.
const PASSIVE = new Set(["get_snapshot", "read_logs", "get_history", "get_sessions", "audit_recent", "lock_status", "check_update", "app_version", "remote_info", "system_info"]);
type Spin = HTMLButtonElement & { __n?: number; __t?: ReturnType<typeof setTimeout> };
let lastBtn: Spin | null = null, lastClick = 0;
let chainBtn: Spin | null = null, chainEnd = 0;

if (typeof document !== "undefined") {
  document.addEventListener("click", (e) => {
    const b = (e.target as Element | null)?.closest?.("button") as Spin | null | undefined;
    if (!b) return;
    if (b.hasAttribute("data-loading")) { e.preventDefault(); e.stopImmediatePropagation(); return; } // déjà en cours
    if (/(^|\s)btn(-primary|-danger)?(\s|$)/.test(b.className)) { lastBtn = b; lastClick = Date.now(); }
  }, true);
}

function trackOnButton(p: Promise<unknown>) {
  const now = Date.now();
  // Le bouton qu'on vient de cliquer, ou celui dont la tâche vient de finir (actions enchaînées : enregistrer puis recharger…).
  const b = lastBtn?.isConnected && now - lastClick < 400 ? lastBtn : chainBtn?.isConnected && now - chainEnd < 350 ? chainBtn : null;
  if (!b) return;
  b.__n = (b.__n ?? 0) + 1;
  if (b.__t) clearTimeout(b.__t);
  b.setAttribute("data-loading", "1");
  b.setAttribute("aria-busy", "true");
  const done = () => {
    b.__n = Math.max(0, (b.__n ?? 1) - 1);
    if (b.__n > 0) return;
    chainBtn = b; chainEnd = Date.now();
    // Léger délai : le rond ne clignote pas pour une tâche instantanée, et la tâche suivante d'une chaîne le reprend.
    b.__t = setTimeout(() => { if ((b.__n ?? 0) === 0) { b.removeAttribute("data-loading"); b.removeAttribute("aria-busy"); } }, 350);
  };
  p.then(done, done);
}

function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const p = callRaw<T>(cmd, args);
  if (!PASSIVE.has(cmd)) trackOnButton(p);
  return p;
}

export async function onSnapshot(cb: (s: Snapshot) => void): Promise<() => void> {
  if (!inTauri) {
    const id = setInterval(async () => cb(await mock<Snapshot>("get_snapshot")), 2000);
    return () => clearInterval(id);
  }
  const { listen } = await import("@tauri-apps/api/event");
  return listen<Snapshot>("snapshot", (e) => cb(e.payload));
}

export async function onInstallLog(cb: (line: string) => void): Promise<() => void> {
  if (!inTauri) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  return listen<string>("install-log", (e) => cb(e.payload));
}

export const api = {
  getSettings: () => call<AppSettings>("get_settings"),
  saveSettings: (settings: AppSettings) => call<void>("save_settings", { settings }),
  start: () => call<void>("server_start"),
  stop: () => call<void>("server_stop"),
  restart: () => call<void>("server_restart"),
  snapshot: () => call<Snapshot>("get_snapshot"),
  readWorld: () => call<WorldSettings>("read_world_settings"),
  writeWorld: (options: Opt[]) => call<void>("write_world_settings", { options }),
  backupNow: () => call<BackupInfo>("backup_now"),
  listBackups: () => call<BackupInfo[]>("list_backups"),
  restoreBackup: (path: string) => call<void>("restore_backup", { path }),
  testAlert: () => call<void>("test_alert"),
  ban: (userId: string, name?: string, reason?: string) => call<void>("ban_player", { userId, name: name ?? null, reason: reason ?? null }),
  playersKnown: () => call<KnownPlayer[]>("players_known"),
  playersBans: () => call<BanEntry[]>("players_bans"),
  unban: (userId: string) => call<void>("unban_player", { userId }),
  updateServer: () => call<string>("update_server"),
  history: (hours: number) => call<Sample[]>("get_history", { hours }),
  sessions: (days: number) => call<Session[]>("get_sessions", { days }),
  logs: (offset: number | null) => call<LogChunk>("read_logs", { offset }),
  detectSetup: () => call<[string[], string[]]>("detect_setup"),
  appVersion: () => call<string>("app_version"),
  checkUpdate: () => call<UpdateInfo | null>("check_update"),
  installUpdate: (info: UpdateInfo) => call<void>("install_update", { info }),
  verifyBackup: (path: string) => call<VerifyReport>("verify_backup", { path }),
  profiles: () => call<ProfileInfo[]>("profiles_list"),
  profileSave: (name: string) => call<void>("profile_save", { name }),
  profileApply: (name: string) => call<number>("profile_apply", { name }),
  profileDelete: (name: string) => call<void>("profile_delete", { name }),
  checkServerUpdate: () => call<ServerUpdateInfo>("check_server_update"),
  useRunningServerDir: () => call<string>("use_running_server_dir"),
  modPackSave: (name: string) => call<void>("mod_pack_save", { name }),
  modPackApply: (name: string) => call<string[]>("mod_pack_apply", { name }),
  modPackDelete: (name: string) => call<void>("mod_pack_delete", { name }),
  installEverything: (baseDir: string) => call<void>("install_everything", { baseDir }),
  upnpTest: () => call<string>("upnp_test"),
  playerSnapshots: (playerId: string) => call<PlayerSnapshot[]>("player_snapshots", { playerId }),
  playerSnapshotNow: (playerId: string) => call<PlayerSnapshot | null>("player_snapshot_now", { playerId }),
  playerRestore: (path: string) => call<string[]>("player_restore", { path }),
  playerExport: (path: string, targetDir?: string) => call<string>("player_export", { path, targetDir: targetDir ?? null }),
  backupNowProtected: (name?: string) => call<BackupInfo>("backup_now_protected", { name: name ?? null }),
  backupSetProtected: (path: string, on: boolean) => call<void>("backup_set_protected", { path, on }),
  auditRecent: (limit?: number) => call<AuditEntry[]>("audit_recent", { limit: limit ?? 200 }),
  lockStatus: () => call<LockStatus>("lock_status"),
  lockVerify: (pin: string) => call<boolean>("lock_verify", { pin }),
  lockSet: (current: string | null, newPin: string | null, autoLockMinutes: number) => call<void>("lock_set", { current, newPin, autoLockMinutes }),
  restoreTestStatus: () => call<RestoreTestView>("restore_test_status"),
  restoreTestNow: () => call<RestoreTestView>("restore_test_now"),
  seasonStatus: () => call<SeasonState>("season_status"),
  diagnose: () => call<Check[]>("diagnose"),
  fixRest: (adminPassword: string) => call<void>("fix_rest", { adminPassword }),
  getAutostart: () => call<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => call<void>("set_autostart", { enabled }),
  networkInfo: () => call<{ lan_ip: string | null }>("network_info"),
  publicIp: () => call<string>("public_ip"),
  systemInfo: () => call<SystemInfo>("system_info"),
  applyPerformance: () => call<number>("apply_performance"),
  remoteInfo: () => call<RemoteInfo>("remote_info"),
  regenerateToken: () => call<void>("regenerate_remote_token"),
  createGuest: (name: string, perms: Perm[], hours: number | null) => call<Guest>("create_guest", { name, perms, hours }),
  revokeGuest: (id: string) => call<void>("revoke_guest", { id }),
  modsState: () => call<ModsState>("mods_state"),
  modsSetGlobal: (enabled: boolean) => call<void>("mods_set_global", { enabled }),
  modsSetRoot: (path: string) => call<void>("mods_set_root", { path }),
  modsSetActive: (packageName: string, active: boolean) => call<void>("mods_set_active", { packageName, active }),
  modsAdd: (input: string) => call<string>("mods_add", { input }),
  modsRemove: (workshopId: string) => call<void>("mods_remove", { workshopId }),
  analyzeLog: () => call<Finding[]>("analyze_log"),
  announce: (message: string) => call<void>("announce", { message }),
  kick: (userId: string) => call<void>("kick_player", { userId }),
};
