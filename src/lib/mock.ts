import type { AppSettings, Opt } from "./types";

const settings: AppSettings = {
  server_dir: "C:\\palworld\\PalServer", steamcmd_path: "steamcmd.exe", performance: { priority: "normal", cpu_cores: null, memory_limit_gb: null, memory_limit_restart: false },
  schedule: { enabled: false, rules: [{ time: "04:00", action: "restart", days: [] }], announce_minutes: [15, 5, 1], memory_restart_percent: null }, launch_args: ["-useperfthreads"], auto_restart: true, close_to_tray: false, start_server_on_launch: false,
  rest: { host: "127.0.0.1", port: 8212, admin_password: "" },
  backup: { enabled: true, interval_minutes: 30, retention: 20, destination: "backups", mirror_destination: null, on_stop: true },
  alerts: { discord_webhook: null, ntfy_url: null, on_crash: true, on_player_join: true, on_player_leave: false, memory_threshold_percent: 90, cooldown_secs: 300, daily_summary_time: null },
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
    case "get_history": {
      const now = Math.floor(Date.now() / 1000);
      return r(Array.from({ length: 144 }, (_, i) => ({ t: now - (143 - i) * 600, cpu: 15 + 10 * Math.sin(i / 9), mem_percent: 40 + i * 0.2, players: Math.max(0, Math.round(3 + 2 * Math.sin(i / 12))), fps: 58 })));
    }
    case "get_sessions": {
      const now = Math.floor(Date.now() / 1000);
      return r([{ name: "Alice", start: now - 5400, end: null }, { name: "Bob", start: now - 9000, end: now - 3600 }]);
    }
    case "read_logs": return r({ lines: ["[demo] Server started", "[demo] Alice joined the game"], offset: 0 });
    case "diagnose": return r([{ id: "exe", label: "PalServer.exe trouvé", ok: true, detail: "" }, { id: "rest_enabled", label: "API REST activée (RESTAPIEnabled=True)", ok: false, detail: "" }]);
    case "get_autostart": return r(false);
    case "network_info": return r({ lan_ip: "192.168.1.42" });
    case "public_ip": return r("203.0.113.7");
    case "system_info": return r({ cpu_cores: 12, total_memory_bytes: 32e9 });
    case "apply_performance": return r(2);
    case "read_world_settings": return r({ options: world, from_default: false });
    case "write_world_settings": world = (args as { options: Opt[] }).options; return r(undefined);
    case "list_backups": return r([{ file_name: "palworld-20260930-120000.zip", path: "backups/x.zip", size_bytes: 52_000_000, created: new Date().toISOString() }]);
    default: return r(undefined);
  }
}
