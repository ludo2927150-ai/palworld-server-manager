// Pont IPC. Hors Tauri (npm run dev dans un navigateur) on bascule sur des données factices.
import type { AppSettings, BackupInfo, Check, LogChunk, Opt, Sample, Session, Snapshot, WorldSettings } from "./types";
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
  ban: (userId: string) => call<void>("ban_player", { userId }),
  unban: (userId: string) => call<void>("unban_player", { userId }),
  updateServer: () => call<string>("update_server"),
  history: (hours: number) => call<Sample[]>("get_history", { hours }),
  sessions: (days: number) => call<Session[]>("get_sessions", { days }),
  logs: (offset: number | null) => call<LogChunk>("read_logs", { offset }),
  diagnose: () => call<Check[]>("diagnose"),
  fixRest: (adminPassword: string) => call<void>("fix_rest", { adminPassword }),
  getAutostart: () => call<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => call<void>("set_autostart", { enabled }),
  networkInfo: () => call<{ lan_ip: string | null }>("network_info"),
  publicIp: () => call<string>("public_ip"),
  announce: (message: string) => call<void>("announce", { message }),
  kick: (userId: string) => call<void>("kick_player", { userId }),
};
