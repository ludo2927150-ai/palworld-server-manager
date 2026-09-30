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
  server_dir: string; launch_args: string[]; auto_restart: boolean;
  rest: { host: string; port: number; admin_password: string };
  backup: { enabled: boolean; interval_minutes: number; retention: number; destination: string };
  alerts: {
    discord_webhook: string | null; ntfy_url: string | null; on_crash: boolean; on_player_join: boolean;
    on_player_leave: boolean; memory_threshold_percent: number | null; cooldown_secs: number;
  };
}
