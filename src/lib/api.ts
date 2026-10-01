// Pont IPC. Hors Tauri (npm run dev dans un navigateur) on bascule sur des données factices.
import type { AppSettings, BackupInfo, Check, BanEntry, VerifyReport, ProfileInfo, ServerUpdateInfo, UpdateInfo, KnownPlayer, Finding, Guest, ModsState, LogChunk, Opt, Perm, RemoteInfo, Sample, Session, Snapshot, SystemInfo, WorldSettings } from "./types";
import { mock } from "./mock";

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) return mock<T>(cmd, args);
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
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
