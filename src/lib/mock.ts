import type { AppSettings, Opt } from "./types";

const settings: AppSettings = {
  server_dir: "C:\\palworld\\PalServer", steamcmd_path: "steamcmd.exe",
  schedule: { enabled: false, times: ["04:00"], announce_minutes: [15, 5, 1], memory_restart_percent: null }, launch_args: ["-useperfthreads"], auto_restart: true,
  rest: { host: "127.0.0.1", port: 8212, admin_password: "" },
  backup: { enabled: true, interval_minutes: 30, retention: 20, destination: "backups" },
  alerts: { discord_webhook: null, ntfy_url: null, on_crash: true, on_player_join: true, on_player_leave: false, memory_threshold_percent: 90, cooldown_secs: 300 },
};
let world: Opt[] = [
  { key: "ServerName", value: "Default Palworld Server", quoted: true },
  { key: "ExpRate", value: "1.000000", quoted: false },
  { key: "bIsPvP", value: "False", quoted: false },
  { key: "ServerPlayerMaxNum", value: "32", quoted: false },
  { key: "DeathPenalty", value: "All", quoted: false },
];

export async function mock<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const r = (v: unknown) => v as T;
  switch (cmd) {
    case "get_settings": return r(settings);
    case "get_snapshot": return r({
      running: true, cpu_percent: 10 + Math.random() * 20, memory_bytes: 9e9, memory_percent: 55 + Math.random() * 5,
      total_memory_bytes: 16e9, metrics: { currentplayernum: 2, maxplayernum: 32, serverfps: 58, days: 12, uptime: 7200 },
      players: [{ name: "Alice", accountName: "alice", playerId: "1", userId: "steam_1", level: 34, ping: 30 }, { name: "Bob", accountName: "bob", playerId: "2", userId: "steam_2", level: 21, ping: 55 }],
    });
    case "read_world_settings": return r(world);
    case "write_world_settings": world = (args as { options: Opt[] }).options; return r(undefined);
    case "list_backups": return r([{ file_name: "palworld-20260930-120000.zip", path: "backups/x.zip", size_bytes: 52_000_000, created: new Date().toISOString() }]);
    default: return r(undefined);
  }
}
