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
  remote: { enabled: boolean; port: number; token: string };
  performance: PerformanceSettings;
  schedule: { enabled: boolean; rules: ScheduleRule[]; announce_minutes: number[]; memory_restart_percent: number | null };
  launch_args: string[]; auto_restart: boolean; close_to_tray: boolean; start_server_on_launch: boolean;
  rest: { host: string; port: number; admin_password: string };
  backup: { enabled: boolean; interval_minutes: number; retention: number; destination: string; mirror_destination: string | null; on_stop: boolean };
  alerts: {
    discord_webhook: string | null; ntfy_url: string | null; on_crash: boolean; on_player_join: boolean;
    on_player_leave: boolean; memory_threshold_percent: number | null; cooldown_secs: number;
    daily_summary_time: string | null;
  };
}
export interface Sample { t: number; cpu: number; mem_percent: number; players: number; fps: number }
export interface Session { name: string; start: number; end: number | null }
export interface LogChunk { lines: string[]; offset: number }
export interface Check { id: string; label: string; ok: boolean; detail: string }
export interface WorldSettings { options: Opt[]; from_default: boolean }
export type RuleAction = "start" | "stop" | "restart";
export interface ScheduleRule { time: string; action: RuleAction; days: number[] }
export type Priority = "belownormal" | "normal" | "abovenormal" | "high";
export interface PerformanceSettings { priority: Priority; cpu_cores: number[] | null; memory_limit_gb: number | null; memory_limit_restart: boolean }
export interface SystemInfo { cpu_cores: number; total_memory_bytes: number }
export interface RemoteUrl { label: string; url: string }
export interface RemoteInfo { running: boolean; error: string | null; urls: RemoteUrl[]; tailscale_found: boolean }
