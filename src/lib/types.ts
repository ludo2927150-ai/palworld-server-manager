// Miroir des structs Rust (crates/core). Garder synchronisé.
export interface Player { name: string; accountName: string; playerId: string; userId: string; level: number; ping: number }
export interface Metrics { currentplayernum: number; maxplayernum: number; serverfps: number; days: number; uptime: number }
export interface Snapshot {
  running: boolean; cpu_percent: number; memory_bytes: number; memory_percent: number;
  total_memory_bytes: number; metrics: Metrics | null; players: Player[];
}
export interface Opt { key: string; value: string; quoted: boolean }
export interface BackupInfo { file_name: string; path: string; size_bytes: number; created: string }
export interface AppSettings {
  server_dir: string; steamcmd_path: string;
  access: AccessSettings;
  announcements: AnnouncementSettings;
  remote: { enabled: boolean; port: number; token: string; guests: Guest[] };
  performance: PerformanceSettings;
  schedule: { enabled: boolean; rules: ScheduleRule[]; announce_minutes: number[]; memory_restart_percent: number | null };
  launch_args: string[]; auto_restart: boolean; close_to_tray: boolean; start_server_on_launch: boolean; setup_done: boolean; log_file: string | null;
  rest: { host: string; port: number; admin_password: string };
  backup: { enabled: boolean; interval_minutes: number; retention: number; destination: string; mirror_destination: string | null; on_stop: boolean };
  alerts: {
    discord_webhook: string | null; ntfy_url: string | null; on_crash: boolean; on_player_join: boolean;
    on_player_leave: boolean; memory_threshold_percent: number | null; cooldown_secs: number;
    daily_summary_time: string | null; desktop: boolean;
    stale_backup_hours: number | null; min_free_disk_gb: number | null;
  };
  server_update: { enabled: boolean; check_every_minutes: number; warn_minutes: number };
  discord_bot: { enabled: boolean; bot_token: string; allowed_user_ids: string[]; allow_control: boolean };
}
export interface Sample { t: number; cpu: number; mem_percent: number; players: number; fps: number }
export interface Session { name: string; start: number; end: number | null }
export interface LogChunk { lines: string[]; offset: number; source: string; hint: string | null }
export interface Check { id: string; label: string; ok: boolean; detail: string }
export interface WorldSettings { options: Opt[]; from_default: boolean }
export type RuleAction = "start" | "stop" | "restart";
export interface ScheduleRule { time: string; action: RuleAction; days: number[] }
export type Priority = "belownormal" | "normal" | "abovenormal" | "high";
export interface PerformanceSettings { priority: Priority; cpu_cores: number[] | null; memory_limit_gb: number | null; memory_limit_restart: boolean }
export interface SystemInfo { cpu_cores: number; total_memory_bytes: number }
export interface RemoteUrl { label: string; url: string }
export interface RemoteInfo { running: boolean; error: string | null; urls: RemoteUrl[]; bases: RemoteUrl[]; tailscale_found: boolean }
export type Perm = "status" | "players" | "logs" | "charts" | "start" | "stop" | "restart" | "backup" | "announce" | "kick";
export interface Guest { id: string; name: string; token: string; perms: Perm[]; expires_at: number | null; created_at: number }
export interface ModInfo {
  workshop_id: string; package_name: string; name: string | null; version: string | null; author: string | null;
  server_compatible: boolean; active: boolean; removable: boolean; path: string;
}
export interface ModsState {
  settings_path: string; global_enable: boolean; workshop_root: string | null; root_exists: boolean;
  candidates: string[]; download_root: string; mods: ModInfo[];
}
export interface Finding { id: string; severity: "critical" | "warning" | "info"; title: string; advice: string; count: number; sample: string }
export interface AnnouncementRule { id: string; text: string; every_minutes: number; enabled: boolean }
export interface AnnouncementSettings { enabled: boolean; only_with_players: boolean; welcome: string | null; rules: AnnouncementRule[] }
export interface AllowedPlayer { user_id: string; name: string }
export interface AccessSettings { whitelist_enabled: boolean; allowed: AllowedPlayer[]; kick_message: string }
export interface KnownPlayer {
  user_id: string; name: string; previous_names: string[]; first_seen: number; last_seen: number; sessions: number; total_secs: number;
  online: boolean; banned: boolean; allowed: boolean;
}
export interface BanEntry { user_id: string; name: string; banned_at: number; reason: string | null }
export interface UpdateInfo { current: string; latest: string; notes: string; url: string; asset: string }
export interface VerifyReport { files: number; bytes: number }
export interface ProfileInfo { name: string; saved_at: number; options: number }
export interface ServerUpdateInfo { installed: string | null; latest: string; outdated: boolean }
