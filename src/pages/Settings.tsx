import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { AppSettings, RuleAction, ScheduleRule } from "../lib/types";

const DAYS = ["L", "M", "M", "J", "V", "S", "D"];
const DAY_NAMES = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"];
const ACTIONS: { v: RuleAction; label: string }[] = [
  { v: "start", label: "Démarrer" }, { v: "restart", label: "Redémarrer" }, { v: "stop", label: "Arrêter" },
];
const validTime = (t: string) => /^([01]?\d|2[0-3]):[0-5]\d$/.test(t.trim());
const padTime = (t: string) => (validTime(t) ? t.trim().replace(/^(\d):/, "0$1:") : t);

function ScheduleEditor({ s, setS }: { s: AppSettings; setS: (v: AppSettings) => void }) {
  const set = (rules: ScheduleRule[]) => setS({ ...s, schedule: { ...s.schedule, rules } });
  const upd = (i: number, patch: Partial<ScheduleRule>) => set(s.schedule.rules.map((r, j) => (j === i ? { ...r, ...patch } : r)));
  const toggleDay = (i: number, d: number) => {
    const cur = s.schedule.rules[i].days.length ? s.schedule.rules[i].days : [0, 1, 2, 3, 4, 5, 6];
    const next = cur.includes(d) ? cur.filter((x) => x !== d) : [...cur, d].sort();
    if (next.length === 0) return;                       // au moins un jour
    upd(i, { days: next.length === 7 ? [] : next });     // vide = tous les jours
  };
  return (
    <div className="card space-y-3">
      <div className="flex items-center gap-3">
        <h2 className="font-semibold">Horaires programmés</h2>
        <label className="ml-auto flex items-center gap-2 text-sm">
          <input type="checkbox" checked={s.schedule.enabled} onChange={(e) => setS({ ...s, schedule: { ...s.schedule, enabled: e.target.checked } })} /> Activés
        </label>
      </div>
      <p className="text-xs text-slate-500">
        Heure locale du PC. Avant un arrêt ou un redémarrage, les joueurs sont prévenus en jeu aux minutes indiquées ci-dessous, et une sauvegarde est faite.
        Si l'application est fermée à l'heure prévue, l'action est ignorée.
      </p>
      {s.schedule.rules.length === 0 && <p className="text-sm text-slate-400">Aucun horaire.</p>}
      {s.schedule.rules.map((r, i) => (
        <div key={i} className="flex flex-wrap items-center gap-2">
          <input className={`input !w-24 ${validTime(r.time) ? "" : "!border-red-500"}`} value={r.time} placeholder="HH:MM" aria-label="Heure (HH:MM)"
            onChange={(e) => upd(i, { time: e.target.value })} onBlur={(e) => upd(i, { time: padTime(e.target.value) })} />
          <select className="input !w-40" value={r.action} aria-label="Action" onChange={(e) => upd(i, { action: e.target.value as RuleAction })}>
            {ACTIONS.map((a) => <option key={a.v} value={a.v}>{a.label}</option>)}
          </select>
          <div className="flex gap-1" role="group" aria-label="Jours">
            {DAYS.map((d, di) => {
              const on = r.days.length === 0 || r.days.includes(di);
              return (
                <button key={di} type="button" aria-pressed={on} title={DAY_NAMES[di]} onClick={() => toggleDay(i, di)}
                  className={`h-8 w-8 rounded-lg text-xs font-medium ${on ? "bg-pal-600 text-white" : "bg-slate-800 text-slate-400"}`}>{d}</button>
              );
            })}
          </div>
          <button type="button" className="btn ml-auto" onClick={() => set(s.schedule.rules.filter((_, j) => j !== i))}>Supprimer</button>
        </div>
      ))}
      <button type="button" className="btn" onClick={() => set([...s.schedule.rules, { time: "04:00", action: "restart", days: [] }])}>+ Ajouter un horaire</button>
    </div>
  );
}

const Row = ({ label, children }: { label: string; children: React.ReactNode }) => (
  <label className="block text-sm"><span className="mb-1 block text-slate-400">{label}</span>{children}</label>
);

export default function Settings({ notify }: { notify: (m: string) => void }) {
  const [s, setS] = useState<AppSettings | null>(null);
  const [autostart, setAutostart] = useState(false);
  const [autostartError, setAutostartError] = useState<string | null>(null);
  useEffect(() => { api.getAutostart().then(setAutostart).catch((e) => setAutostartError(String(e))); }, []);
  // Coche tout de suite, puis annule et affiche l'erreur réelle si Windows refuse.
  const toggleAutostart = (on: boolean) => {
    setAutostart(on); setAutostartError(null);
    api.setAutostart(on).catch((e) => { setAutostart(!on); setAutostartError(String(e)); });
  };
  useEffect(() => { api.getSettings().then(setS).catch((e) => notify(String(e))); }, [notify]);
  if (!s) return <p className="text-slate-400">Chargement…</p>;
  const nul = (v: string) => (v.trim() ? v : null);

  return (
    <div className="space-y-4">
      <div className="card grid gap-4 md:grid-cols-2">
        <Row label="Dossier du serveur (PalServer.exe)"><input className="input" value={s.server_dir} onChange={(e) => setS({ ...s, server_dir: e.target.value })} /></Row>
        <Row label="Arguments de lancement"><input className="input" value={s.launch_args.join(" ")} onChange={(e) => setS({ ...s, launch_args: e.target.value.split(" ").filter(Boolean) })} /></Row>
        <Row label="Mot de passe admin (API REST)"><input className="input" type="password" value={s.rest.admin_password} onChange={(e) => setS({ ...s, rest: { ...s.rest, admin_password: e.target.value } })} /></Row>
        <Row label="Lancer avec Windows (réduit dans la zone de notification)">
          <input type="checkbox" checked={autostart} onChange={(e) => toggleAutostart(e.target.checked)} />
          <span className="ml-2 text-xs text-slate-500">à utiliser avec l'application installée (pas avec « npm run tauri dev »)</span>
          {autostartError && <span className="mt-1 block text-xs text-red-400" role="alert">{autostartError}</span>}
        </Row>
        <Row label="Démarrer le serveur automatiquement au lancement de l'application (≈ 15 s après)"><input type="checkbox" checked={s.start_server_on_launch} onChange={(e) => setS({ ...s, start_server_on_launch: e.target.checked })} /></Row>
        <Row label="Fermer la fenêtre = réduire dans la zone de notification"><input type="checkbox" checked={s.close_to_tray} onChange={(e) => setS({ ...s, close_to_tray: e.target.checked })} /></Row>
        <Row label="Redémarrage auto après crash"><input type="checkbox" checked={s.auto_restart} onChange={(e) => setS({ ...s, auto_restart: e.target.checked })} /></Row>
      </div>
      <ScheduleEditor s={s} setS={setS} />
      <div className="card grid gap-4 md:grid-cols-2">
        <Row label="Chemin de steamcmd.exe"><input className="input" value={s.steamcmd_path} onChange={(e) => setS({ ...s, steamcmd_path: e.target.value })} /></Row>
        <Row label="Annonces (minutes avant, ex. 15, 5, 1)"><input className="input" value={s.schedule.announce_minutes.join(", ")} onChange={(e) => setS({ ...s, schedule: { ...s.schedule, announce_minutes: e.target.value.split(",").map((t) => parseInt(t, 10)).filter((n) => n > 0) } })} /></Row>
        <Row label="Redémarrer si RAM ≥ (%) — vide = jamais"><input className="input" type="number" min={1} max={100} value={s.schedule.memory_restart_percent ?? ""} onChange={(e) => setS({ ...s, schedule: { ...s.schedule, memory_restart_percent: e.target.value ? +e.target.value : null } })} /></Row>
      </div>
      <div className="card grid gap-4 md:grid-cols-2">
        <Row label="Backup automatique"><input type="checkbox" checked={s.backup.enabled} onChange={(e) => setS({ ...s, backup: { ...s.backup, enabled: e.target.checked } })} /></Row>
        <Row label="Intervalle (minutes)"><input className="input" type="number" min={1} value={s.backup.interval_minutes} onChange={(e) => setS({ ...s, backup: { ...s.backup, interval_minutes: +e.target.value } })} /></Row>
        <Row label="Nombre de backups conservés"><input className="input" type="number" min={1} value={s.backup.retention} onChange={(e) => setS({ ...s, backup: { ...s.backup, retention: +e.target.value } })} /></Row>
        <Row label="Second emplacement (copie de chaque sauvegarde, ex. autre disque)"><input className="input" placeholder="D:\Sauvegardes\Palworld" value={s.backup.mirror_destination ?? ""} onChange={(e) => setS({ ...s, backup: { ...s.backup, mirror_destination: e.target.value.trim() ? e.target.value : null } })} /></Row>
        <Row label="Dossier de destination"><input className="input" value={s.backup.destination} onChange={(e) => setS({ ...s, backup: { ...s.backup, destination: e.target.value } })} /></Row>
      </div>
      <div className="card grid gap-4 md:grid-cols-2">
        <Row label="Webhook Discord"><input className="input" value={s.alerts.discord_webhook ?? ""} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, discord_webhook: nul(e.target.value) } })} /></Row>
        <Row label="URL ntfy (push mobile)"><input className="input" placeholder="https://ntfy.sh/mon-topic" value={s.alerts.ntfy_url ?? ""} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, ntfy_url: nul(e.target.value) } })} /></Row>
        <Row label="Résumé quotidien à (HH:MM, vide = désactivé)"><input className="input" placeholder="20:00" value={s.alerts.daily_summary_time ?? ""} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, daily_summary_time: nul(e.target.value) } })} /></Row>
        <Row label="Seuil mémoire (%)"><input className="input" type="number" min={1} max={100} value={s.alerts.memory_threshold_percent ?? ""} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, memory_threshold_percent: e.target.value ? +e.target.value : null } })} /></Row>
        <Row label="Alertes"><div className="flex gap-4">
          {([["on_crash", "Crash"], ["on_player_join", "Connexion"], ["on_player_leave", "Déconnexion"]] as const).map(([k, l]) => (
            <label key={k}><input type="checkbox" checked={s.alerts[k]} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, [k]: e.target.checked } })} /> {l}</label>
          ))}
        </div></Row>
      </div>
      <div className="flex gap-2">
        <button className="btn-primary" onClick={() => s.schedule.rules.some((r) => !validTime(r.time)) ? notify("Un horaire programmé n'a pas un format HH:MM valide") : api.saveSettings(s).then(() => notify("Paramètres enregistrés")).catch((e) => notify(String(e)))}>Enregistrer</button>
        <button className="btn" onClick={() => api.testAlert().then(() => notify("Alerte de test envoyée")).catch((e) => notify(String(e)))}>Tester les alertes</button>
      </div>
    </div>
  );
}
