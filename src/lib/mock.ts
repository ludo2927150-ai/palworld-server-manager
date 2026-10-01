import type { AppSettings, Opt } from "./types";

const settings: AppSettings = {
  server_dir: "C:\\palworld\\PalServer", steamcmd_path: "steamcmd.exe", access: { whitelist_enabled: false, allowed: [{ user_id: "steam_1", name: "Alice" }], kick_message: "" }, announcements: { enabled: true, only_with_players: true, welcome: "Bienvenue {nom} ! Ici : {joueurs}/{max} joueurs.", rules: [{ id: "a1", text: "Pensez à faire une pause et à boire de l'eau !", every_minutes: 30, enabled: true }], personal: [], welcome_back_days: 0, welcome_back_text: null }, remote: { enabled: true, port: 8765, token: "0123456789abcdef0123456789abcdef", guests: [{ id: "g1", name: "Alice", token: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", perms: ["status", "players"], expires_at: null, created_at: 0 }] }, performance: { priority: "normal", cpu_cores: null, memory_limit_gb: null, memory_limit_restart: false },
  schedule: { enabled: false, rules: [{ time: "04:00", action: "restart", days: [], profile: null }], announce_minutes: [15, 5, 1], memory_restart_percent: null, memory_restart_wait_empty_minutes: 0, events: [] }, launch_args: ["-useperfthreads"], auto_restart: true, close_to_tray: false, start_server_on_launch: false, setup_done: true, log_file: null, capture_console: true,
  rest: { host: "127.0.0.1", port: 8212, admin_password: "" },
  backup: { enabled: true, interval_minutes: 30, retention: 20, destination: "backups", mirror_destination: null, on_stop: true, tiered: false, player_snapshots: true, player_keep: 10, restore_test_days: 7 },
  alerts: { discord_webhook: null, ntfy_url: null, on_crash: true, on_player_join: true, on_player_leave: false, memory_threshold_percent: 90, cooldown_secs: 300, daily_summary_time: null, desktop: true, weekly_report_day: null, stale_backup_hours: 6, min_free_disk_gb: 5 },
  watchdog: { enabled: true, hung_minutes: 5, crash_loop_max: 3, crash_loop_window_minutes: 10 },
  mod_automation: { managed_ids: [], auto_update: false, check_every_minutes: 120, packs: [] },
  upnp: { enabled: false },
  lock: { enabled: false, salt: "", hash: "", auto_lock_minutes: 0 },
  server_update: { enabled: false, check_every_minutes: 60, warn_minutes: 5 },
  discord_bot: { enabled: false, bot_token: "", allowed_user_ids: [], allow_control: false },
};
let world: Opt[] = [
  { key: "ServerName", value: "Default Palworld Server", quoted: true },
  { key: "ExpRate", value: "1.000000", quoted: false },
  { key: "bIsPvP", value: "False", quoted: false },
  { key: "ServerPlayerMaxNum", value: "32", quoted: false },
  { key: "DeathPenalty", value: "All", quoted: false },
];

const demoMods = [
  { workshop_id: "3123456789", package_name: "FastHandiwork", name: "Fast Handiwork", version: "1.2", author: "Moddeur", server_compatible: true, active: true, removable: true, path: "x" },
  { workshop_id: "3123456790", package_name: "CoolSkins", name: "Cool Skins", version: "2.0", author: "Artiste", server_compatible: false, active: false, removable: false, path: "y" },
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
    case "read_logs": return r({ lines: ["[demo] Server started", "[demo] Alice joined the game"], offset: 0, source: "Pal/Saved/Logs/Pal.log", hint: null });
    case "detect_setup": return r([["C:/SteamLibrary/steamapps/common/PalServer"], ["C:/steamcmd/steamcmd.exe"]]);
    case "app_version": return r("0.1.0-test.1");
    case "check_update": return r(null);
    case "verify_backup": return r({ files: 214, bytes: 48_000_000 });
    case "profiles_list": return r([{ name: "Normal", saved_at: Math.floor(Date.now() / 1000) - 86400, options: 90 }, { name: "Hardcore", saved_at: Math.floor(Date.now() / 1000) - 3600, options: 90 }]);
    case "profile_save": case "profile_delete": return r(undefined);
    case "profile_apply": return r(7);
    case "check_server_update": return r({ installed: "15012345", latest: "15012345", outdated: false });
    case "use_running_server_dir": return r("C:/palworld/PalServer");
    case "mod_pack_save": case "mod_pack_delete": return r(undefined);
    case "mod_pack_apply": return r([]);
    case "install_everything": return r(undefined);
    case "upnp_test": return r("Port UDP 8211 ouvert (démo)");
    case "player_snapshots": return r([{ file_name: "20261001-180000-1a2b3c4d.zip", path: "x/1.zip", taken: new Date(Date.now() - 3600e3).toISOString(), size_bytes: 48210 }, { file_name: "20260930-210000-9f8e7d6c.zip", path: "x/2.zip", taken: new Date(Date.now() - 86400e3).toISOString(), size_bytes: 47990 }]);
    case "player_snapshot_now": return r(null);
    case "player_restore": return r(["F8A7388F000000000000000000000000.sav"]);
    case "player_export": return r("C:/Users/demo/Desktop/joueur-F8A7388F-demo.zip");
    case "backup_now_protected": return r({ file_name: "palworld-20261001-180000-garde-boss.zip", path: "x/g.zip", size_bytes: 48e6, created: new Date().toISOString(), protected: true });
    case "backup_set_protected": return r(undefined);
    case "audit_recent": return r([
      { t: Math.floor(Date.now() / 1000) - 120, who: "mobile:Alice", action: "serveur : Restart", detail: "" },
      { t: Math.floor(Date.now() / 1000) - 3600, who: "auto:maj", action: "cycle de maintenance", detail: "✅ Serveur redémarré (serveur mis à jour)." },
      { t: Math.floor(Date.now() / 1000) - 7200, who: "app", action: "sauvegarde manuelle", detail: "" },
    ]);
    case "lock_status": return r({ enabled: false, auto_lock_minutes: 0 });
    case "lock_verify": return r(true);
    case "lock_set": return r(undefined);
    case "restore_test_status": return r({ t: Math.floor(Date.now() / 1000) - 86400 * 2, ok: true, detail: "palworld-20261001-040000-auto.zip : 214 fichiers, 48.0 Mo, 3 joueur(s)" });
    case "restore_test_now": return r({ t: Math.floor(Date.now() / 1000), ok: true, detail: "palworld-20261001-040000-auto.zip : 214 fichiers, 48.0 Mo, 3 joueur(s)" });
    case "season_status": return r({ applied: null, return_profile: null });
    case "diagnose": return r([{ id: "exe", label: "PalServer.exe trouvé", ok: true, detail: "" }, { id: "rest_enabled", label: "API REST activée (RESTAPIEnabled=True)", ok: false, detail: "" }]);
    case "get_autostart": return r(false);
    case "network_info": return r({ lan_ip: "192.168.1.42" });
    case "public_ip": return r("203.0.113.7");
    case "system_info": return r({ cpu_cores: 12, total_memory_bytes: 32e9 });
    case "apply_performance": return r(2);
    case "remote_info": return r({ running: true, error: null, tailscale_found: true, bases: [{ label: "Partout (Tailscale, 4G ou Wi-Fi)", url: "http://100.101.102.103:8765/" }, { label: "Chez vous (même Wi-Fi)", url: "http://192.168.1.42:8765/" }], urls: [{ label: "Partout (Tailscale, 4G ou Wi-Fi)", url: "http://100.101.102.103:8765/#token=0123456789abcdef0123456789abcdef" }, { label: "Chez vous (même Wi-Fi)", url: "http://192.168.1.42:8765/#token=0123456789abcdef0123456789abcdef" }] });
    case "create_guest": return r({ id: "g2", name: (args as { name: string }).name, token: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", perms: (args as { perms: string[] }).perms, expires_at: null, created_at: 0 });
    case "mods_state": return r({
      settings_path: "C:\\palworld\\Mods\\PalModSettings.ini", global_enable: true, workshop_root: "C:\\palworld\\steamapps\\workshop\\content\\1623730", root_exists: true,
      candidates: ["C:\\palworld\\steamapps\\workshop\\content\\1623730"], download_root: "C:\\palworld\\steamapps\\workshop\\content\\1623730",
      mods: demoMods });
    case "mods_set_active": { const a = args as { packageName: string; active: boolean }; demoMods.forEach((x) => { if (x.package_name === a.packageName) x.active = a.active; }); return r(undefined); }
    case "mods_add": return r("Mod 3123456791 téléchargé.");
    case "analyze_log": return r([
      { id: "oom", severity: "critical", title: "Mémoire insuffisante", advice: "Le serveur a manqué de RAM. Dans l'onglet Performance, fixez une limite de RAM avec redémarrage automatique.", count: 2, sample: "Failed to allocate 4294967296 bytes" },
      { id: "mod", severity: "warning", title: "Erreur liée à un mod", advice: "Un mod pose problème. Dans l'onglet Mods, désactivez-le puis redémarrez.", count: 1, sample: "[WARN] mod error in X" },
    ]);
    case "players_known": { const n = Math.floor(Date.now() / 1000); return r([
      { user_id: "steam_1", name: "Alice", previous_names: ["Ali"], first_seen: n - 86400 * 20, last_seen: n, sessions: 41, total_secs: 3600 * 63, player_id: "F8A7388F000000000000000000000000", level: 31, max_level: 31, location: [-182340, 95210], buildings: 42, level_history: [[Math.floor(Date.now() / 1000) - 86400 * 20, 1], [Math.floor(Date.now() / 1000) - 86400 * 10, 15], [Math.floor(Date.now() / 1000) - 86400, 31]], online: true, banned: false, allowed: true },
      { user_id: "steam_2", name: "Bob", previous_names: [], first_seen: n - 86400 * 9, last_seen: n - 3600 * 5, sessions: 12, total_secs: 3600 * 18, player_id: "0123456789ABCDEF0123456789ABCDEF", level: 12, max_level: 12, location: null, buildings: 0, level_history: [], online: false, banned: false, allowed: false },
      { user_id: "steam_3", name: "Troll", previous_names: ["Gentil"], first_seen: n - 86400 * 2, last_seen: n - 86400, sessions: 2, total_secs: 900, player_id: "", level: 3, max_level: 3, location: null, buildings: 0, level_history: [], online: false, banned: true, allowed: false },
    ]); }
    case "players_bans": return r([{ user_id: "steam_3", name: "Troll", banned_at: Math.floor(Date.now() / 1000) - 86400, reason: "Grief" }]);
    case "read_world_settings": return r({ options: world, from_default: false });
    case "write_world_settings": world = (args as { options: Opt[] }).options; return r(undefined);
    case "list_backups": return r([{ file_name: "palworld-20260930-120000.zip", path: "backups/x.zip", size_bytes: 52_000_000, created: new Date().toISOString(), protected: false }]);
    default: return r(undefined);
  }
}
