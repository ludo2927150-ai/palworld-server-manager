import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { AppSettings } from "../lib/types";

const Row = ({ label, children }: { label: string; children: React.ReactNode }) => (
  <label className="block text-sm"><span className="mb-1 block text-slate-400">{label}</span>{children}</label>
);

export default function Settings({ notify }: { notify: (m: string) => void }) {
  const [s, setS] = useState<AppSettings | null>(null);
  useEffect(() => { api.getSettings().then(setS).catch((e) => notify(String(e))); }, [notify]);
  if (!s) return <p className="text-slate-400">Chargement…</p>;
  const nul = (v: string) => (v.trim() ? v : null);

  return (
    <div className="space-y-4">
      <div className="card grid gap-4 md:grid-cols-2">
        <Row label="Dossier du serveur (PalServer.exe)"><input className="input" value={s.server_dir} onChange={(e) => setS({ ...s, server_dir: e.target.value })} /></Row>
        <Row label="Arguments de lancement"><input className="input" value={s.launch_args.join(" ")} onChange={(e) => setS({ ...s, launch_args: e.target.value.split(" ").filter(Boolean) })} /></Row>
        <Row label="Mot de passe admin (API REST)"><input className="input" type="password" value={s.rest.admin_password} onChange={(e) => setS({ ...s, rest: { ...s.rest, admin_password: e.target.value } })} /></Row>
        <Row label="Redémarrage auto après crash"><input type="checkbox" checked={s.auto_restart} onChange={(e) => setS({ ...s, auto_restart: e.target.checked })} /></Row>
      </div>
      <div className="card grid gap-4 md:grid-cols-2">
        <Row label="Backup automatique"><input type="checkbox" checked={s.backup.enabled} onChange={(e) => setS({ ...s, backup: { ...s.backup, enabled: e.target.checked } })} /></Row>
        <Row label="Intervalle (minutes)"><input className="input" type="number" min={1} value={s.backup.interval_minutes} onChange={(e) => setS({ ...s, backup: { ...s.backup, interval_minutes: +e.target.value } })} /></Row>
        <Row label="Nombre de backups conservés"><input className="input" type="number" min={1} value={s.backup.retention} onChange={(e) => setS({ ...s, backup: { ...s.backup, retention: +e.target.value } })} /></Row>
        <Row label="Dossier de destination"><input className="input" value={s.backup.destination} onChange={(e) => setS({ ...s, backup: { ...s.backup, destination: e.target.value } })} /></Row>
      </div>
      <div className="card grid gap-4 md:grid-cols-2">
        <Row label="Webhook Discord"><input className="input" value={s.alerts.discord_webhook ?? ""} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, discord_webhook: nul(e.target.value) } })} /></Row>
        <Row label="URL ntfy (push mobile)"><input className="input" placeholder="https://ntfy.sh/mon-topic" value={s.alerts.ntfy_url ?? ""} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, ntfy_url: nul(e.target.value) } })} /></Row>
        <Row label="Seuil mémoire (%)"><input className="input" type="number" min={1} max={100} value={s.alerts.memory_threshold_percent ?? ""} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, memory_threshold_percent: e.target.value ? +e.target.value : null } })} /></Row>
        <Row label="Alertes"><div className="flex gap-4">
          {([["on_crash", "Crash"], ["on_player_join", "Connexion"], ["on_player_leave", "Déconnexion"]] as const).map(([k, l]) => (
            <label key={k}><input type="checkbox" checked={s.alerts[k]} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, [k]: e.target.checked } })} /> {l}</label>
          ))}
        </div></Row>
      </div>
      <div className="flex gap-2">
        <button className="btn-primary" onClick={() => api.saveSettings(s).then(() => notify("Paramètres enregistrés")).catch((e) => notify(String(e)))}>Enregistrer</button>
        <button className="btn" onClick={() => api.testAlert().then(() => notify("Alerte de test envoyée")).catch((e) => notify(String(e)))}>Tester les alertes</button>
      </div>
    </div>
  );
}
